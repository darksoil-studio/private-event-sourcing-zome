import { ActionCommittedSignal } from '@darksoil-studio/holochain-utils';
import {
	AgentPubKey,
	EntryHash,
	Signature,
	Timestamp,
} from '@holochain/client';

export type PrivateEventSourcingSignal =
	| ActionCommittedSignal<EntryTypes, LinkTypes>
	| {
			type: 'NewPrivateEvent';
			event_hash: EntryHash;
			private_event_entry: PrivateEventEntry;
	  };

export type EntryTypes =
	| ({ type: 'PrivateEvent' } & PrivateEventEntry)
	| ({ type: 'EventSentToRecipients' } & EventSentToRecipients)
	| ({ type: 'Acknowledgement' } & Acknowledgement);

export type LinkTypes = string;

export interface SignedEventContent<T> {
	event_type: string;
	event: T;
}

export interface SignedContent<T> {
	timestamp: Timestamp;
	content: T;
}

export interface SignedEntry<T> {
	author: AgentPubKey;
	signature: Signature;
	payload: SignedContent<T>;
}

export type SignedEvent<T> = SignedEntry<SignedEventContent<T>>;
export type PrivateEventEntry = SignedEvent<Uint8Array>;

export type EventSentToRecipients = SignedEntry<{
	event_hash: EntryHash;
	recipients: Array<AgentPubKey>;
}>;

export type Acknowledgement = SignedEntry<{
	private_event_hash: EntryHash;
}>;
