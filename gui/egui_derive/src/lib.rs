use proc_macro::TokenStream;
use quote::quote;

/// Generate i18n helper implementations from enum variant doc comments.
///
/// # Generated
///
/// - `I18nKey::ALL`
/// - `I18nKey::default_eng()`
///
/// # Rules
///
/// - Each variant must have a `///` doc comment.
/// - Multi-line docs are joined with `\n`.
/// - Only supports field-less enums.
///
/// # Example
///
/// ```rust
/// # egui::derive::I18n
///
/// #[derive(I18n)]
/// enum I18nKey {
///     /// Execute
///     ExecuteButton,
///
///     /// Cancel
///     CancelButton,
///
///     /// Internal invalid key
///     Invalid,
/// }
/// ```
///
/// Expands roughly to:
///
/// ```rust
/// impl I18nKey {
///     pub const ALL: &'static [Self] = &[
///         Self::ExecuteButton,
///         Self::CancelButton,
///         Self::Invalid
///     ];
///
///     pub const fn default_eng(&self) -> &'static str {
///         match self {
///             Self::ExecuteButton => "Execute",
///             Self::CancelButton => "Cancel",
///             Self::Invalid => "Internal invalid key",
///         }
///     }
/// }
/// ```
///
/// # Panics
/// If invalid syntax as Rust
#[proc_macro_derive(I18n, attributes(i18n))]
pub fn derive_i18n(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    let enum_name = &input.ident;

    let syn::Data::Enum(data_enum) = &input.data else {
        panic!("I18n only supports enums");
    };

    let mut doc_lines = vec![
        //
        "Generated from enum variant doc comments.\n".to_string(),
        "# Keys\n\n".to_string(),
        "| Key | Default |".to_string(),
        "|-----|---------|".to_string(),
    ];

    let mut variants = Vec::new();
    let mut match_arms = Vec::new();

    for variant in &data_enum.variants {
        let ident = &variant.ident;

        variants.push(quote! {
            Self::#ident
        });

        let mut docs = Vec::new();

        for attr in &variant.attrs {
            if attr.path().is_ident("doc")
                && let syn::Meta::NameValue(meta) = &attr.meta
                && let syn::Expr::Lit(expr) = &meta.value
                && let syn::Lit::Str(lit) = &expr.lit
            {
                docs.push(lit.value().trim().to_string());
            }
        }

        if docs.is_empty() {
            panic!("missing doc comment for variant `{ident}`");
        }

        let joined = docs.join("\n");

        match_arms.push(quote! {
            Self::#ident => #joined
        });

        fn escape_md_table_cell(s: &str) -> String {
            s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('|', "\\|")
        }
        doc_lines.push(format!("| `{ident}` | {} |", escape_md_table_cell(&joined)));
    }

    let generated_doc = doc_lines.join("\n");
    quote! {
        impl #enum_name {
            pub const ALL: &'static [Self] = &[
                #(#variants),*
            ];

            #[doc = #generated_doc]
            pub const fn default_eng(&self) -> &'static str {
                match self {
                    #(#match_arms),*
                }
            }
        }
    }
    .into()
}
