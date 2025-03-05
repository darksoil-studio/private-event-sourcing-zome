import { createContext } from '@lit/context';
import { PrivateEventSourcingStore } from './private-event-sourcing-store.js';

export const privateEventSourcingStoreContext = createContext<PrivateEventSourcingStore>(
  'private_event_sourcing/store'
);

