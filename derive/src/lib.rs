//! Custom derive support for the `simple-tlv` crate

#![crate_type = "proc-macro"]
#![warn(rust_2018_idioms, trivial_casts, unused_qualifications)]

mod decodable;
use decodable::DeriveDecodableStruct;
mod encodable;
use encodable::DeriveEncodableStruct;


use proc_macro2::TokenStream;
use syn::{
    Attribute, Field, Ident, Lit, Meta, MetaList, MetaNameValue, NestedMeta,
};
use synstructure::{decl_derive, Structure};

decl_derive!(
    [Decodable, attributes(tlv)] =>

    /// Derive the [`Decodable`][1] trait on a struct.
    ///
    /// See [toplevel documentation for the `simple-tlv_derive` crate][2] for more
    /// information about how to use this macro.
    ///
    /// [1]: https://docs.rs/simple-tlv/latest/simple_tlv/trait.Decodable.html
    /// [2]: https://docs.rs/simple-tlv_derive/
    derive_decodable
);

decl_derive!(
    [Encodable, attributes(tlv)] =>

    /// Derive the [`Encodable`][1] trait on a struct.
    ///
    /// See [toplevel documentation for the `simple-tlv_derive` crate][2] for more
    /// information about how to use this macro.
    ///
    /// [1]: https://docs.rs/simple-tlv/latest/simple_tlv/trait.Decodable.html
    /// [2]: https://docs.rs/simple-tlv_derive/
    derive_encodable
);

/// Custom derive for `simple_tlv::Decodable`
fn derive_decodable(s: Structure<'_>) -> TokenStream {
    let ast = s.ast();

    // TODO: enum support
    match &ast.data {
        syn::Data::Struct(data) => DeriveDecodableStruct::derive(s, data, &ast.ident, &ast.attrs),
        other => panic!("can't derive `Decodable` on: {:?}", other),
    }
}

/// Custom derive for `simple_tlv::Encodable`
fn derive_encodable(s: Structure<'_>) -> TokenStream {
    let ast = s.ast();

    // TODO: enum support
    match &ast.data {
        syn::Data::Struct(data) => DeriveEncodableStruct::derive(s, data, &ast.ident, &ast.attrs),
        other => panic!("can't derive `Encodable` on: {:?}", other),
    }
}

/// Attributes of a field
#[derive(Debug)]
struct FieldAttrs {
    /// Name of the field
    pub name: Ident,

    /// Value of the `#[asn1(type = "...")]` attribute if provided
    pub tag: u8,
}

impl FieldAttrs {
    /// Parse the attributes of a field
    fn new(field: &Field) -> Self {
        let name = field
            .ident
            .as_ref()
            .cloned()
            .expect("no name on struct field i.e. tuple structs unsupported");

        let tag = extract_tag(&name, &field.attrs);

        Self { name, tag }
    }
}

fn extract_tag(name: &Ident, attrs: &Vec<Attribute>) -> u8 {
    let mut tag = None;

    for attr in attrs {
        if !attr.path.is_ident("tlv") {
            continue;
        }

        match attr.parse_meta().expect("error parsing `tlv` attribute") {
            Meta::List(MetaList { nested, .. }) if nested.len() == 1 => {
                match nested.first() {
                    Some(NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                        path,
                        lit: Lit::Str(lit_str),
                        ..
                    }))) => {
                        // Parse the `type = "..."` attribute
                        if !path.is_ident("tag") {
                            panic!("unknown `tlv` attribute for field `{}`: {:?}", name, path);
                        }

                        if tag.is_some() {
                            panic!("duplicate SIMPLE-TLV `tag` attribute for field: {}", name);
                        }

                        let possibly_with_prefix = lit_str.value();
                        let without_prefix = possibly_with_prefix.trim_start_matches("0x");
                        let tag_value = u8::from_str_radix(without_prefix, 16).expect("tag values must be between one and 254");
                        if tag_value == 0 || tag_value == 255 {
                            panic!("SIMPLE-TLV tags must not be zero or 255");
                        }
                        tag = Some(tag_value);
                    }
                    other => panic!(
                        "malformed `tlv` attribute for field `{}`: {:?}",
                        name, other
                    ),
                }
            }
            other => panic!(
                "malformed `tlv` attribute for field `{}`: {:?}",
                name, other
            ),
        }
    }

    if let Some(tag) = tag {
        tag
    } else {
        panic!("SIMPLE-TLV tag missing for `{}`", name);
    }
}
