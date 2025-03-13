import { EntryHashB64, encodeHashToBase64 } from '@holochain/client';
import { decode } from '@msgpack/msgpack';
import { AsyncComputed } from '@tnesh-stack/signals';

import { PrivateEventSourcingClient } from './private-event-sourcing-client.js';
import { PrivateEventEntry, SignedEvent } from './types.js';
import { asyncReadable } from './utils.js';

export class PrivateEventSourcingStore<E> {
	constructor(public client: PrivateEventSourcingClient<object>) {}

	privateEventEntries = asyncReadable<Record<EntryHashB64, PrivateEventEntry>>(
		async set => {
			const entries = await this.client.queryPrivateEventEntries();
			set(entries ? entries : {});

			return this.client.onSignal(signal => {
				if (!('type' in signal) || signal.type !== 'NewPrivateEvent') return;

				entries[encodeHashToBase64(signal.event_hash)] =
					signal.private_event_entry;
				set(entries);
			});
		},
	);

	privateEvents = new AsyncComputed(() => {
		const privateEventEntries = this.privateEventEntries.get();

		if (privateEventEntries.status !== 'completed') return privateEventEntries;

		const privateEvents: Record<EntryHashB64, SignedEvent<E>> = {};

		for (const [entryHash, privateEventEntry] of Object.entries(
			privateEventEntries.value,
		)) {
			privateEvents[entryHash] = {
				...privateEventEntry,
				event: {
					timestamp: privateEventEntry.event.timestamp,
					content: decode(privateEventEntry.event.content) as E,
				},
			};
		}

		return {
			status: 'completed',
			value: privateEvents,
		};
	});
}
