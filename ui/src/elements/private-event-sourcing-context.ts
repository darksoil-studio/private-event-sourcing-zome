import { css, html, LitElement } from 'lit';
import { provide, consume } from '@lit/context';
import { customElement, property } from 'lit/decorators.js';
import { AppClient } from '@holochain/client';
import { appClientContext } from '@tnesh-stack/elements';

import { privateEventSourcingStoreContext } from '../context.js';
import { PrivateEventSourcingStore } from '../private-event-sourcing-store.js';
import { PrivateEventSourcingClient } from '../private-event-sourcing-client.js';

/**
 * @element private-event-sourcing-context
 */
@customElement('private-event-sourcing-context')
export class PrivateEventSourcingContext extends LitElement {
  @consume({ context: appClientContext })
  private client!: AppClient;

  @provide({ context: privateEventSourcingStoreContext })
  @property({ type: Object })
  store!: PrivateEventSourcingStore;

  @property()
  role!: string;

  @property()
  zome = 'private_event_sourcing';

  connectedCallback() {
    super.connectedCallback();
    if (this.store) return;
    if (!this.role) {
      throw new Error(`<private-event-sourcing-context> must have a role="YOUR_DNA_ROLE" property, eg: <private-event-sourcing-context role="role1">`);
    }
    if (!this.client) {
      throw new Error(`<private-event-sourcing-context> must either:
        a) be placed inside <app-client-context>
          or 
        b) receive an AppClient property (eg. <private-event-sourcing-context .client=\${client}>) 
          or 
        c) receive a store property (eg. <private-event-sourcing-context .store=\${store}>)
      `);
    }

    this.store = new PrivateEventSourcingStore(new PrivateEventSourcingClient(this.client, this.role, this.zome));
  }
  
  render() {
    return html`<slot></slot>`;
  }

  static styles = css`
    :host {
      display: contents;
    }
  `;
}

