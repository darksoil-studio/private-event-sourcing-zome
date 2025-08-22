import { toPromise } from '@darksoil-studio/holochain-signals';
import { dhtSync, pause, runScenario } from '@holochain/tryorama';
import { assert, expect, test } from 'vitest';

import { setup, waitUntil } from './setup.js';

test('shared entries get synchronized with a new recipient', async () => {
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

		await carol.store.client.client.callZome({
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

		await carol.store.client.client.callZome({
			role_name: 'private_event_sourcing_test',
			zome_name: 'example',
			fn_name: 'add_friend',
			payload: alice.player.agentPubKey,
		});

		await waitUntil(async () => {
			const alicePrivateEvents = await toPromise(alice.store.privateEvents);
			const carolPrivateEvents = await toPromise(carol.store.privateEvents);
			return (
				Object.keys(alicePrivateEvents).length === 3 &&
				Object.keys(carolPrivateEvents).length === 3
			);
		}, 10_000);
	});
});
