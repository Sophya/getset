use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::{Ident, Span};
use proc_macro_error::{abort, ResultExt};
use syn::{self, ext::IdentExt, spanned::Spanned, Field, Lit, Meta, MetaNameValue, Visibility};

use crate::extract_type_from_option::extract_type_from_option;

use self::GenMode::*;
use super::parse_attr;

pub struct GenParams {
    pub mode: GenMode,
    pub global_attr: Option<Meta>,
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum GenMode {
    Get,
    GetCopy,
    Set,
    GetMut,
    GetExpect,
}

impl GenMode {
    pub fn name(self) -> &'static str {
        match self {
            Get => "get",
            GetCopy => "get_copy",
            Set => "set",
            GetMut => "get_mut",
            GetExpect => "get_expect",
        }
    }

    pub fn prefix(self) -> &'static str {
        match self {
            Get | GetCopy | GetMut | GetExpect => "",
            Set => "set_",
        }
    }

    pub fn suffix(self) -> &'static str {
        match self {
            Get | GetCopy | Set | GetExpect => "",
            GetMut => "_mut",
        }
    }

    fn is_get(self) -> bool {
        match self {
            GenMode::Get | GenMode::GetCopy | GenMode::GetMut | GenMode::GetExpect => true,
            GenMode::Set => false,
        }
    }
}

pub fn parse_visibility(attr: Option<&Meta>, meta_name: &str) -> Option<Visibility> {
    match attr {
        // `#[get = "pub"]` or `#[set = "pub"]`
        Some(Meta::NameValue(MetaNameValue {
            lit: Lit::Str(ref s),
            path,
            ..
        })) => {
            if path.is_ident(meta_name) {
                s.value().split(' ').find(|v| *v != "with_prefix").map(|v| {
                    syn::parse_str(v)
                        .map_err(|e| syn::Error::new(s.span(), e))
                        .expect_or_abort("invalid visibility found")
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Some users want legacy/compatability.
/// (Getters are often prefixed with `get_`)
fn has_prefix_attr(f: &Field, params: &GenParams) -> bool {
    let inner = f
        .attrs
        .iter()
        .filter_map(|v| parse_attr(v, params.mode))
        .filter(|meta| {
            ["get", "get_copy", "get_mut", "get_expect"]
                .iter()
                .any(|ident| meta.path().is_ident(ident))
        })
        .last();

    // Check it the attr includes `with_prefix`
    let wants_prefix = |possible_meta: &Option<Meta>| -> bool {
        match possible_meta {
            Some(Meta::NameValue(meta)) => {
                if let Lit::Str(lit_str) = &meta.lit {
                    // Naive tokenization to avoid a possible visibility mod named `with_prefix`.
                    lit_str.value().split(' ').any(|v| v == "with_prefix")
                } else {
                    false
                }
            }
            _ => false,
        }
    };

    // `with_prefix` can either be on the local or global attr
    wants_prefix(&inner) || wants_prefix(&params.global_attr)
}

fn get_type(field: &Field) -> syn::Type {
    if let Some(ty) = extract_type_from_option(&field.ty) {
        ty.clone()
    } else {
        field.ty.clone()
    }
}

pub fn implement(field: &Field, params: &GenParams) -> TokenStream2 {
    let field_name = field
        .clone()
        .ident
        .unwrap_or_else(|| abort!(field.span(), "Expected the field to have a name"));

    let fn_name = if !has_prefix_attr(field, params)
        && (params.mode.is_get())
        && params.mode.suffix().is_empty()
        && field_name.to_string().starts_with("r#")
    {
        field_name.clone()
    } else {
        Ident::new(
            &format!(
                "{}{}{}{}",
                if has_prefix_attr(field, params) && (params.mode.is_get()) {
                    "get_"
                } else {
                    ""
                },
                params.mode.prefix(),
                field_name.unraw(),
                params.mode.suffix()
            ),
            Span::call_site(),
        )
    };
    let fn_name_string = fn_name.to_string();
    let ty = get_type(field);

    let doc = field.attrs.iter().filter(|v| {
        v.parse_meta()
            .map(|meta| meta.path().is_ident("doc"))
            .unwrap_or(false)
    });

    let attr = field
        .attrs
        .iter()
        .filter_map(|v| parse_attr(v, params.mode))
        .last()
        .or_else(|| params.global_attr.clone());

    let visibility = parse_visibility(attr.as_ref(), params.mode.name());
    match attr {
        Some(_) => match params.mode {
            GenMode::Get => {
                quote! {
                    #(#doc)*
                    #[inline(always)]
                    #visibility fn #fn_name(&self) -> &#ty {
                        &self.#field_name
                    }
                }
            }
            GenMode::GetCopy => {
                quote! {
                    #(#doc)*
                    #[inline(always)]
                    #[allow(clone_on_copy)]
                    #visibility fn #fn_name(&mut self) -> #ty {
                        self.#field_name.clone()
                    }
                }
            }
            GenMode::Set => {
                quote! {
                    #(#doc)*
                    #[inline(always)]
                    #visibility fn #fn_name(&mut self, val: &mut #ty) -> &mut Self {
                        self.#field_name = val.clone();
                        self
                    }
                }
            }
            GenMode::GetMut => {
                quote! {
                    #(#doc)*
                    #[inline(always)]
                    #visibility fn #fn_name(&mut self) -> &mut #ty {
                        &mut self.#field_name
                    }
                }
            }
            GenMode::GetExpect => {
                quote! {
                  #(#doc)*
                      #[inline(always)]
                      #visibility fn #fn_name(&self) -> &#ty {
                          self.#field_name.as_ref().expect(&format!("Could not get {}", #fn_name_string))
                      }
                }
            }
        },
        // Don't need to do anything.
        None => quote! {},
    }
}
