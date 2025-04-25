import { toPromise } from '@darksoil-studio/holochain-signals';
import {
	LinkedDevicesClient,
	LinkedDevicesStore,
} from '@darksoil-studio/linked-devices-zome';
import { CellId, CellType } from '@holochain/client';
import {
	AgentApp,
	dhtSync,
	enableAndGetAgentApp,
	pause,
	runScenario,
} from '@holochain/tryorama';
import { assert, expect, test } from 'vitest';

import { PrivateEventSourcingClient } from '../../ui/src/private-event-sourcing-client.js';
import { PrivateEventSourcingStore } from '../../ui/src/private-event-sourcing-store.js';
import { setup, testHappUrl, waitUntil } from './setup.js';

test('create a shared entry gets to each source chain', async () => {
	await runScenario(async scenario => {
		const [alice, bob] = await setup(scenario);

		await alice.store.client.client.callZome({
			role_name: 'private_event_sourcing_test',
			zome_name: 'example',
			fn_name: 'create_private_shared_entry',
			payload: {
				type: 'SharedEntry',
				recipient: bob.player.agentPubKey,
				content: 'hello',
			},
		});

		const appInfo = await alice.player.conductor.installApp(
			{
				type: 'path',
				value: testHappUrl,
			},
			{
				agentPubKey: alice.player.cells[0].cell_id[1],
				networkSeed: 'second app',
			},
		);
		const port = await alice.player.conductor.attachAppInterface();
		const issued = await alice.player.conductor
			.adminWs()
			.issueAppAuthenticationToken({
				installed_app_id: appInfo.installed_app_id,
			});
		const appWs = await alice.player.conductor.connectAppWs(issued.token, port);
		const agentApp: AgentApp = await enableAndGetAgentApp(
			alice.player.conductor.adminWs(),
			appWs,
			appInfo,
		);

		const previousAppInfo = await alice.store.client.client.appInfo();
		const cellInfo =
			previousAppInfo.cell_info['private_event_sourcing_test'][0];
		if (cellInfo.type !== 'provisioned')
			throw new Error('Unexpected cell type.');

		const previousCellId: CellId = cellInfo.value.cell_id;

		await appWs.callZome({
			role_name: 'private_event_sourcing_test',
			zome_name: 'example',
			payload: previousCellId,
			fn_name: 'migrate_from_old_cell',
		});

		await pause(200);
		const aliceStore2 = new PrivateEventSourcingStore(
			new PrivateEventSourcingClient(
				appWs,
				'private_event_sourcing_test',
				'example',
			),
			new LinkedDevicesStore(
				new LinkedDevicesClient(appWs, 'private_event_sourcing_test'),
			),
		);

		let privateEvents = await toPromise(aliceStore2.privateEvents);
		assert.equal(Object.keys(privateEvents).length, 1);
	});
});
