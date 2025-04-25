import {
	AgentPubKey,
	EntryHash,
	Signature,
	Timestamp,
} from '@holochain/client';
import { ActionCommittedSignal } from '@darksoil-studio/holochain-utils';

export type PrivateEventSourcingSignal =
	| ActionCommittedSignal<EntryTypes, LinkTypes>
	| {
			type: 'NewPrivateEvent';
			event_hash: EntryHash;
			private_event_entry: PrivateEventEntry;
	  };

export type EntryTypes = { type: 'PrivateEvent' } & PrivateEventEntry;

export type LinkTypes = string;

export interface SignedContent<T> {
	timestamp: Timestamp;
	content: T;
}

export interface SignedEvent<T> {
	author: AgentPubKey;
	signature: Signature;
	event: SignedContent<T>;
}

export type PrivateEventEntry = SignedEvent<Uint8Array>;
