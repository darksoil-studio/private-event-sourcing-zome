import { toPromise } from '@darksoil-studio/holochain-signals';
import { dhtSync, pause, runScenario } from '@holochain/tryorama';
import { assert, test } from 'vitest';

import { setup, waitUntil } from './setup.js';

test('big entries get gossiped asynchronously', async () => {
	await runScenario(async scenario => {
		const [alice, bob, carol] = await setup(scenario, 3);

		await bob.player.conductor.shutDown();

		const LENGTH = 200_000;

		await alice.store.client.client.callZome({
			role_name: 'private_event_sourcing_test',
			zome_name: 'example',
			fn_name: 'create_private_shared_entry',
			payload: {
				type: 'SharedEntry',
				recipient: bob.player.agentPubKey,
				content: Array.from(Array(LENGTH)).fill('a').join(''),
			},
		});

		let privateEvents = await toPromise(alice.store.privateEvents);
		assert.equal(Object.keys(privateEvents).length, 1);

		await dhtSync(
			[alice.player, carol.player],
			alice.player.cells[0].cell_id[0],
		);

		await alice.player.conductor.shutDown();

		await bob.startUp();

		await waitUntil(async () => {
			const privateEvents = await toPromise(bob.store.privateEvents);
			return Object.keys(privateEvents).length === 1;
		}, 1_000_000);
	});
});
