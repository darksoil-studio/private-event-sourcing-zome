use proc_macro::TokenStream;
use proc_macro_error::abort;
use syn::parse_macro_input;
use syn::Item;
use syn::ItemEnum;

// #[proc_macro_error]
#[proc_macro_attribute]
pub fn private_event(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Item);
    // let attr_args: proc_macro2::TokenStream = attrs.into();

    let ident = match &input {
        Item::Enum(ItemEnum { ident, .. }) => ident,
        _ => abort!(input, "private_event can only be used on Enums"),
    };

    let output = quote::quote! {
        #[derive(Serialize, Deserialize, SerializedBytes, Debug, private_event_sourcing::IntoStaticStr, Clone)] #input

        impl private_event_sourcing::EventType for #ident {
            fn event_type(&self) -> String {
                let s: &'static str = self.into();
                s.to_string()
            }
        }

        #[hdk_extern]
        pub fn attempt_commit_awaiting_deps_entries() -> ExternResult<()> {
            private_event_sourcing::attempt_commit_awaiting_deps_entries::<#ident>()?;

            Ok(())
        }

        #[hdk_extern]
        pub fn send_events(events_hashes: BTreeSet<EntryHash>) -> ExternResult<()> {
            private_event_sourcing::send_events::<#ident>(events_hashes)
        }

        #[hdk_extern]
        pub fn commit_my_pending_encrypted_messages() -> ExternResult<()> {
            private_event_sourcing::commit_my_pending_encrypted_messages::<#ident>()
        }

        #[hdk_extern(infallible)]
        fn scheduled_tasks(_: Option<Schedule>) -> Option<Schedule> {
            if let Err(err) = private_event_sourcing::scheduled_tasks::<#ident>() {
                error!("Failed to perform scheduled tasks: {err:?}");
            }

            Some(Schedule::Persisted("*/30 * * * * * *".into())) // Every 30 seconds
        }
    };
    output.into()
}
