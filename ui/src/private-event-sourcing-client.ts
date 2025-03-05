import { 
  SignedActionHashed,
  CreateLink,
  Link,
  DeleteLink,
  Delete,
  AppClient, 
  Record, 
  ActionHash, 
  EntryHash, 
  AgentPubKey,
} from '@holochain/client';
import { EntryRecord, ZomeClient } from '@tnesh-stack/utils';

import { PrivateEventSourcingSignal } from './types.js';

export class PrivateEventSourcingClient extends ZomeClient<PrivateEventSourcingSignal> {

  constructor(public client: AppClient, public roleName: string, public zomeName = 'private_event_sourcing') {
    super(client, roleName, zomeName);
  }
}
