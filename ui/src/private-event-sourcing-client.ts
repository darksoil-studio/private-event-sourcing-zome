import { ZomeClient } from '@darksoil-studio/holochain-utils';
import { AgentPubKey, AppClient, EntryHashB64 } from '@holochain/client';

import {
	Acknowledgement,
	EventSentToRecipients,
	PrivateEventEntry,
	PrivateEventSourcingSignal,
} from './types.js';

export class PrivateEventSourcingClient<ADDITIONAL_SIGNALS> extends ZomeClient<
	PrivateEventSourcingSignal | ADDITIONAL_SIGNALS
> {
	constructor(
		public client: AppClient,
		public roleName: string,
		public zomeName: string,
	) {
		super(client, roleName, zomeName);
	}

	resendEventsIfNecessary(): Promise<void> {
		return this.callZome('resend_events_if_necessary', undefined);
	}

	queryPrivateEventEntries(): Promise<Record<EntryHashB64, PrivateEventEntry>> {
		return this.callZome('query_private_event_entries', undefined);
	}

	queryEventsSentToRecipientsEntries(): Promise<Array<EventSentToRecipients>> {
		return this.callZome('query_events_sent_to_recipients_entries', undefined);
	}

	queryAcknowledgementEntries(): Promise<Array<Acknowledgement>> {
		return this.callZome('query_acknowledgement_entries', undefined);
	}

	synchronizeWithLinkedDevice(linkedDevice: AgentPubKey) {
		return this.callZome('synchronize_with_linked_device', linkedDevice);
	}
}
