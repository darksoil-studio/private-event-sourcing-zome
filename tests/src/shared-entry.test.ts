import { toPromise } from '@darksoil-studio/holochain-signals';
import { pause, runScenario } from '@holochain/tryorama';
import { assert, expect, test } from 'vitest';

import { setup, waitUntil } from './setup.js';
import { dhtSync } from './sync.js';

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

		let privateEvents = await toPromise(alice.store.privateEvents);
		assert.equal(Object.keys(privateEvents).length, 1);

		await pause(100);

		let eventsSent = await toPromise(alice.store.eventsSentToRecipients);
		assert.equal(Object.keys(eventsSent).length, 1);

		await pause(2000);

		privateEvents = await toPromise(bob.store.privateEvents);
		assert.equal(Object.keys(privateEvents).length, 1);

		eventsSent = await toPromise(bob.store.eventsSentToRecipients);
		assert.equal(Object.keys(eventsSent).length, 1);

		let acknowledgements = await toPromise(alice.store.acknowledgements);
		assert.equal(Object.keys(acknowledgements).length, 1);

		acknowledgements = await toPromise(bob.store.acknowledgements);
		assert.equal(Object.keys(acknowledgements).length, 1);
	});
});

test('create a shared entry gets to each source chain asynchronously', async () => {
	await runScenario(async scenario => {
		const [alice, bob, carol] = await setup(scenario, 3);

		await bob.player.conductor.shutDown();

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

		let privateEvents = await toPromise(alice.store.privateEvents);
		assert.equal(Object.keys(privateEvents).length, 1);

		await dhtSync(
			[alice.player, carol.player],
			alice.player.cells[0].cell_id[0],
			2000,
			1000 * 60 * 10, // 10 mins
		);

		await alice.player.conductor.shutDown();

		await bob.startUp();

		await waitUntil(async () => {
			const privateEvents = await toPromise(bob.store.privateEvents);
			return Object.keys(privateEvents).length === 1;
		}, 200_000);
	});
});
