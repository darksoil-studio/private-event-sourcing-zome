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
    };
    output.into()
}
