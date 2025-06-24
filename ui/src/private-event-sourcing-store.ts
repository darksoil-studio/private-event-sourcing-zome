import {
	AsyncComputed,
	mapCompleted,
} from '@darksoil-studio/holochain-signals';
import { HashType, retype } from '@darksoil-studio/holochain-utils';
import { LinkedDevicesStore } from '@darksoil-studio/linked-devices-zome';
import {
	AgentPubKeyB64,
	EntryHashB64,
	encodeHashToBase64,
} from '@holochain/client';
import { decode } from '@msgpack/msgpack';

import { PrivateEventSourcingClient } from './private-event-sourcing-client.js';
import {
	Acknowledgement,
	EventSentToRecipients,
	PrivateEventEntry,
	SignedEvent,
} from './types.js';
import { asyncReadable } from './utils.js';

export class PrivateEventSourcingStore<E> {
	constructor(
		public client: PrivateEventSourcingClient<object>,
		public linkedDevicesStore?: LinkedDevicesStore,
	) {
		if (linkedDevicesStore) {
			linkedDevicesStore.client.onSignal(signal => {
				if (signal.type !== 'LinkCreated') return;
				if (signal.link_type !== 'AgentToLinkedDevices') return;
				this.client.synchronizeWithLinkedDevice(
					retype(signal.action.hashed.content.target_address, HashType.AGENT),
				);
			});
		}
	}

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
				payload: {
					timestamp: privateEventEntry.payload.timestamp,
					content: {
						event_type: privateEventEntry.payload.content.event_type,
						event: decode(privateEventEntry.payload.content.event) as E,
					},
				},
			};
		}

		return {
			status: 'completed',
			value: privateEvents,
		};
	});

	private eventsSentToRecipientsEntries = asyncReadable<
		Array<EventSentToRecipients>
	>(async set => {
		let eventsSentToRecipients =
			await this.client.queryEventsSentToRecipientsEntries();
		set(eventsSentToRecipients);

		return this.client.onSignal(signal => {
			if (!('type' in signal) || signal.type !== 'EntryCreated') return;
			if (signal.app_entry.type !== 'EventSentToRecipients') return;

			eventsSentToRecipients.push(signal.app_entry);
			set(eventsSentToRecipients);
		});
	});

	eventsSentToRecipients = mapCompleted(
		this.eventsSentToRecipientsEntries,
		entries => {
			const eventsSentToRecipients: Record<
				EntryHashB64,
				Record<AgentPubKeyB64, number>
			> = {};

			const sorted = entries.sort(
				(e1, e2) => e1.payload.timestamp - e2.payload.timestamp,
			);

			for (const entry of sorted) {
				if (
					!eventsSentToRecipients[
						encodeHashToBase64(entry.payload.content.event_hash)
					]
				) {
					eventsSentToRecipients[
						encodeHashToBase64(entry.payload.content.event_hash)
					] = {};
				}

				for (const recipient of entry.payload.content.recipients) {
					eventsSentToRecipients[
						encodeHashToBase64(entry.payload.content.event_hash)
					][encodeHashToBase64(recipient)] = entry.payload.timestamp / 1000;
				}
			}

			return eventsSentToRecipients;
		},
	);

	private acknowledgementEntries = asyncReadable<Array<Acknowledgement>>(
		async set => {
			let acknowledgements = await this.client.queryAcknowledgementEntries();
			set(acknowledgements);

			return this.client.onSignal(signal => {
				if (!('type' in signal) || signal.type !== 'EntryCreated') return;
				if (signal.app_entry.type !== 'Acknowledgement') return;

				acknowledgements.push(signal.app_entry);
				set(acknowledgements);
			});
		},
	);

	acknowledgements = mapCompleted(this.acknowledgementEntries, entries => {
		const acknowledgements: Record<
			EntryHashB64,
			Record<AgentPubKeyB64, number>
		> = {};

		const sorted = entries.sort(
			(e1, e2) => e1.payload.timestamp - e2.payload.timestamp,
		);

		for (const entry of sorted) {
			if (
				!acknowledgements[
					encodeHashToBase64(entry.payload.content.private_event_hash)
				]
			) {
				acknowledgements[
					encodeHashToBase64(entry.payload.content.private_event_hash)
				] = {};
			}

			acknowledgements[
				encodeHashToBase64(entry.payload.content.private_event_hash)
			][encodeHashToBase64(entry.author)] = entry.payload.timestamp / 1000;
		}

		return acknowledgements;
	});
}
