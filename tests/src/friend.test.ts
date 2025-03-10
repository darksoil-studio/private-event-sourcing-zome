import { dhtSync, pause, runScenario } from '@holochain/tryorama';
import { toPromise } from '@tnesh-stack/signals';
import { assert, expect, test } from 'vitest';

import { setup, waitUntil } from './setup.js';

test('a shared entry gets eventually synchronized with a new recipient', async () => {
	await runScenario(async scenario => {
		const [alice, bob, carol] = await setup(scenario, 3);

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

		await alice.store.client.client.callZome({
			role_name: 'private_event_sourcing_test',
			zome_name: 'example',
			fn_name: 'add_friend',
			payload: carol.player.agentPubKey,
		});

		await waitUntil(async () => {
			const privateEvents = await toPromise(carol.store.privateEvents);
			return Object.keys(privateEvents).length === 1;
		}, 40_000);
	});
});
