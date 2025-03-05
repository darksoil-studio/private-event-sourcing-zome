import {
	ActionHash,
	AgentPubKey,
	AppClient,
	CreateLink,
	Delete,
	DeleteLink,
	EntryHash,
	EntryHashB64,
	Link,
	SignedActionHashed,
} from '@holochain/client';
import { EntryRecord, ZomeClient } from '@tnesh-stack/utils';

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
}
