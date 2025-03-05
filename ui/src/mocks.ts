import {
	ActionHash,
	AgentPubKey,
	AppClient,
	Delete,
	EntryHash,
	Link,
	NewEntryAction,
	Record,
	SignedActionHashed,
	decodeHashFromBase64,
	fakeActionHash,
	fakeAgentPubKey,
	fakeDnaHash,
	fakeEntryHash,
} from '@holochain/client';
import {
	AgentPubKeyMap,
	HashType,
	HoloHashMap,
	ZomeMock,
	decodeEntry,
	fakeCreateAction,
	fakeDeleteEntry,
	fakeEntry,
	fakeRecord,
	fakeUpdateEntry,
	hash,
	pickBy,
} from '@tnesh-stack/utils';

import { PrivateEventSourcingClient } from './private-event-sourcing-client.js';

export class PrivateEventSourcingZomeMock
	extends ZomeMock
	implements AppClient
{
	constructor(myPubKey?: AgentPubKey) {
		super(
			'private_event_sourcing_test',
			'private_event_sourcing',
			'private_event_sourcing_test_app',
			myPubKey,
		);
	}
}
