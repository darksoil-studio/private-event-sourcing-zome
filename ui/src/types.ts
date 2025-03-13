import { AgentPubKey, Signature, Timestamp } from '@holochain/client';
import { ActionCommittedSignal } from '@tnesh-stack/utils';

export type PrivateEventSourcingSignal = ActionCommittedSignal<
	EntryTypes,
	LinkTypes
>;

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
