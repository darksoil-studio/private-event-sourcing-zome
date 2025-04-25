import { dhtSync, pause, runScenario } from '@holochain/tryorama';
import { toPromise } from '@darksoil-studio/holochain-signals';
import { assert, expect, test } from 'vitest';

import { linkDevices, setup, waitUntil } from './setup.js';

test('create a shared entry gets to linked-devices', async () => {
	await runScenario(async scenario => {
		const [alice, alice2, bob] = await setup(scenario, 3);

		await alice.store.client.client.callZome({
			role_name: 'private_event_sourcing_test',
			zome_name: 'example',
			fn_name: 'create_private_shared_entry',
			payload: {
				type: 'NewFriend',
				friend: bob.player.agentPubKey,
			},
		});

		let privateEvents = await toPromise(alice.store.privateEvents);
		assert.equal(Object.keys(privateEvents).length, 1);

		await pause(2000);

		privateEvents = await toPromise(alice2.store.privateEvents);
		assert.equal(Object.keys(privateEvents).length, 0);

		await linkDevices(
			alice.store.linkedDevicesStore,
			alice2.store.linkedDevicesStore,
		);

		await pause(2000);

		privateEvents = await toPromise(alice2.store.privateEvents);
		assert.equal(Object.keys(privateEvents).length, 1);
	});
});
