import { dhtSync, pause, runScenario } from '@holochain/tryorama';
import { toPromise } from '@tnesh-stack/signals';
import { assert, expect, test } from 'vitest';

import { setup } from './setup.js';

test('create a shared entry', async () => {
	await runScenario(async scenario => {
		const [alice, bob] = await setup(scenario);

		await alice.store.client.client.callZome({
			role_name: 'private_event_sourcing_test',
			zome_name: 'example',
			fn_name: 'create_private_shared_entry',
			payload: {
				recipient: bob.player.agentPubKey,
				content: 'hello',
			},
		});

		let privateEvents = await toPromise(alice.store.privateEvents);
		assert.equal(Object.keys(privateEvents).length, 1);

		await pause(2000);

		privateEvents = await toPromise(bob.store.privateEvents);
		assert.equal(Object.keys(privateEvents).length, 1);
	});
});
