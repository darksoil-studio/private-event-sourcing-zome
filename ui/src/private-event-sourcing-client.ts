import { AgentPubKey, AppClient, EntryHashB64 } from '@holochain/client';
import { ZomeClient } from '@tnesh-stack/utils';

import { PrivateEventEntry, PrivateEventSourcingSignal } from './types.js';

export class PrivateEventSourcingClient extends ZomeClient<PrivateEventSourcingSignal> {
	constructor(
		public client: AppClient,
		public roleName: string,
		public zomeName: string,
	) {
		super(client, roleName, zomeName);
	}

	queryPrivateEventEntries(): Promise<Record<EntryHashB64, PrivateEventEntry>> {
		return this.callZome('query_private_event_entries', undefined);
	}

	synchronizeWithLinkedDevice(linkedDevice: AgentPubKey) {
		return this.callZome('synchronize_with_linked_device', undefined);
	}
}
