//! Custom derive support for the `flexiber` crate
//!
//! With `#[tlv(slice)]` set, `Encodable` should work for fields implementing `AsRef<[u8]>`,
//! and `Decodable` should work for fields implementing `TryFrom<[u8]>`, even if the field
//! is not `Decodable` or `Encodable`.

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
    /// See [toplevel documentation for the `flexiber_derive` crate][2] for more
    /// information about how to use this macro.
    ///
    /// [1]: https://docs.rs/flexiber/latest/flexiber/trait.Decodable.html
    /// [2]: https://docs.rs/flexiber_derive/
    derive_decodable
);

decl_derive!(
    [Encodable, attributes(tlv)] =>

    /// Derive the [`Encodable`][1] trait on a struct.
    ///
    /// See [toplevel documentation for the `flexiber_derive` crate][2] for more
    /// information about how to use this macro.
    ///
    /// [1]: https://docs.rs/flexiber/latest/flexiber/trait.Decodable.html
    /// [2]: https://docs.rs/flexiber_derive/
    derive_encodable
);

/// Custom derive for `flexiber::Decodable`
fn derive_decodable(s: Structure<'_>) -> TokenStream {
    let ast = s.ast();

    // TODO: enum support
    match &ast.data {
        syn::Data::Struct(data) => DeriveDecodableStruct::derive(s, data, &ast.ident, &ast.attrs),
        other => panic!("can't derive `Decodable` on: {:?}", other),
    }
}

/// Custom derive for `flexiber::Encodable`
fn derive_encodable(s: Structure<'_>) -> TokenStream {
    let ast = s.ast();

    // TODO: enum support
    match &ast.data {
        syn::Data::Struct(data) => DeriveEncodableStruct::derive(s, data, &ast.ident, &ast.attrs),
        other => panic!("can't derive `Encodable` on: {:?}", other),
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct BerTag {
    class: Class,
    constructed: bool,
    number: u16,
}

#[derive(Clone, Copy, Debug, Default)]
struct SimpleTag(u8);

#[derive(Clone, Copy, Debug)]
enum Tag {
    Ber(BerTag),
    Simple(SimpleTag),
}

impl From<BerTag> for Tag {
    fn from(tag: BerTag) -> Self {
        Self::Ber(tag)
    }
}

impl From<SimpleTag> for Tag {
    fn from(tag: SimpleTag) -> Self {
        Self::Simple(tag)
    }
}

impl Default for Tag {
    fn default() -> Self {
        Self::Ber(BerTag::default())
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
enum Class {
    Universal = 0b00,
    Application = 0b01,
    Context = 0b10,
    Private = 0b11,
}

impl Default for Class {
    fn default() -> Self {
        Class::Universal
    }
}

/// Attributes of a field
#[derive(Debug)]
struct FieldAttrs {
    /// Name of the field
    pub name: Ident,

    /// Value of tag to use
    pub tag: Tag,

    /// Whether the `#[tlv(slice)]` attribute was set
    pub slice: bool
}

impl FieldAttrs {
    /// Parse the attributes of a field
    fn new(field: &Field) -> Self {
        let name = field
            .ident
            .as_ref()
            .cloned()
            .expect("no name on struct field i.e. tuple structs unsupported");

        let (tag, slice) = extract_attrs(&name, &field.attrs);

        Self { name, tag, slice }
    }
}

fn extract_attrs_optional_tag(name: &Ident, attrs: &[Attribute]) -> (Option<Tag>, bool) {
    let mut tag = Tag::default();
    let mut tag_number_is_set = false;
    let mut slice = false;

    for attr in attrs {
        if !attr.path.is_ident("tlv") {
            continue;
        }

        match attr.parse_meta().expect("error parsing `tlv` attribute") {
            Meta::List(MetaList { nested, .. }) if !nested.is_empty() => {
                for entry in nested {
                    match entry {
                        NestedMeta::Meta(Meta::Path(path)) => {
                            if path.is_ident("slice") {
                                slice = true;
                            } else if path.is_ident("universal") {
                                tag = {
                                    let mut tag = if let Tag::Ber(tag) = tag {
                                        tag
                                    } else { Default::default() };
                                    tag.class = Class::Universal;
                                    tag.into()
                                };
                            } else if path.is_ident("application") {
                                tag = {
                                    let mut tag = if let Tag::Ber(tag) = tag {
                                        tag
                                    } else { Default::default() };
                                    tag.class = Class::Application;
                                    tag.into()
                                };
                            } else if path.is_ident("context") {
                                tag = {
                                    let mut tag = if let Tag::Ber(tag) = tag {
                                        tag
                                    } else { Default::default() };
                                    tag.class = Class::Context;
                                    tag.into()
                                };
                            } else if path.is_ident("private") {
                                tag = {
                                    let mut tag = if let Tag::Ber(tag) = tag {
                                        tag
                                    } else { Default::default() };
                                    tag.class = Class::Private;
                                    tag.into()
                                };
                            } else if path.is_ident("constructed") {
                                tag = {
                                    let mut tag = if let Tag::Ber(tag) = tag {
                                        tag
                                    } else { Default::default() };
                                    tag.constructed = true;
                                    tag.into()
                                };
                            } else if path.is_ident("primitive") {
                                tag = {
                                    let mut tag = if let Tag::Ber(tag) = tag {
                                        tag
                                    } else { Default::default() };
                                    tag.constructed = false;
                                    tag.into()
                                };
                            } else {
                                panic!("unknown `tlv` attribute for field `{}`: {:?}", name, path);
                            }
                        }
                        NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                            path,
                            lit: Lit::Str(lit_str),
                            ..
                        })) => {
                            // Parse the `type = "..."` attribute
                            if path.is_ident("number") {
                                tag = {
                                    let possibly_with_prefix = lit_str.value();
                                    let without_prefix = possibly_with_prefix.trim_start_matches("0x");
                                    let tag_number = u16::from_str_radix(without_prefix, 16).expect("tag values must be between one and 254");
                                    let mut tag = if let Tag::Ber(tag) = tag {
                                        tag
                                    } else { Default::default() };
                                    tag.number = tag_number;
                                    tag_number_is_set = true;
                                    tag.into()
                                }
                            } else if path.is_ident("simple") {
                                tag = {
                                    let possibly_with_prefix = lit_str.value();
                                    let without_prefix = possibly_with_prefix.trim_start_matches("0x");
                                    let tag_number = u8::from_str_radix(without_prefix, 16).expect("tag values must be between one and 254");
                                    let mut tag = if let Tag::Simple(tag) = tag {
                                        tag
                                    } else { Default::default() };
                                    tag.0 = tag_number;
                                    tag_number_is_set = true;
                                    tag.into()
                                };
                            } else {
                                panic!("unknown `tlv` attribute for field `{}`: {:?}", name, path);
                            }

                        }
                        other => panic!(
                            "a malformed `tlv` attribute for field `{}`: {:?}",
                            name, other
                        ),
                    }
                }
            }
            other => panic!(
                "malformed `tlv` attribute for field `{}`: {:#?}",
                name, other
            ),
        }
    }

    if tag_number_is_set {
        (Some(tag), slice)
    } else {
        (None, slice)
    }
}

fn extract_attrs(name: &Ident, attrs: &[Attribute]) -> (Tag, bool) {
    let (tag, slice) = extract_attrs_optional_tag(name, attrs);

    if let Some(tag) = tag {
        (tag, slice)
    } else {
        panic!("BER-TLV tag missing for `{}`", name);
    }
}
