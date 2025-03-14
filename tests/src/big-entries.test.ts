import { pause, runScenario } from '@holochain/tryorama';
import { toPromise } from '@tnesh-stack/signals';
import { assert, test } from 'vitest';

import { setup, waitUntil } from './setup.js';

test('big entries get gossiped asynchronously', async () => {
	await runScenario(async scenario => {
		const [alice, bob, _carol] = await setup(scenario, 3);

		await bob.player.conductor.shutDown();

		const LENGTH = 400_000;

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

		await pause(2000);

		await alice.player.conductor.shutDown();

		await bob.startUp();

		await waitUntil(async () => {
			const privateEvents = await toPromise(bob.store.privateEvents);
			return Object.keys(privateEvents).length === 1;
		}, 100_000);
	});
});
