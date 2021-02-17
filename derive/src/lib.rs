//! Custom derive support for the `simple-tlv` crate

#![crate_type = "proc-macro"]
#![warn(rust_2018_idioms, trivial_casts, unused_qualifications)]

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    Attribute, DataStruct, Field, Generics, Ident, Lifetime, Lit, Meta, MetaList, MetaNameValue, NestedMeta,
};
use synstructure::{decl_derive, Structure};

decl_derive!(
    [UntaggedCollection, attributes(tlv)] =>

    /// Derive the `Message` trait.
    ///
    /// This custom derive macro can be used to automatically impl the
    /// `Message` trait for any struct representing a message which is
    /// encoded as an ASN.1 `SEQUENCE`.
    ///
    /// # `#[asn1(type = "...")]` attribute
    ///
    /// Placing this attribute on fields of a struct makes it possible to
    /// decode types which don't directly implement the `Decode` and `Encode`
    /// traits but do impl `TryInto` and `From` for one of the ASN.1 types
    /// listed below:
    ///
    /// - `bit-string`: performs an intermediate conversion to `der::BitString`
    /// - `octet-string`: performs an intermediate conversion to `der::OctetString`
    /// - `printable-string`: performs an intermediate conversion to `der::PrintableString`
    /// - `utf8-string`: performs an intermediate conversion to `der::Utf8String`
    ///
    /// Note: please open a GitHub Issue if you would like to request support
    /// for additional ASN.1 types.
    derive_simple_tlv
);

/// Custom derive for `der::Message`
fn derive_simple_tlv(s: Structure<'_>) -> TokenStream {
    let ast = s.ast();

    // TODO(tarcieri/nickray): enum support
    match &ast.data {
        syn::Data::Struct(data) => DeriveStruct::derive(s, data, &ast.ident, &ast.attrs, &ast.generics),
        other => panic!("can't derive `Message` on: {:?}", other),
    }
}

/// Derive stuff on a struct
struct DeriveStruct {
    /// Field decoders
    decode_fields: TokenStream,

    /// Bound fields of a struct to be returned
    decode_result: TokenStream,

    /// Fields of a struct to be serialized
    encode_fields: TokenStream,
}

impl DeriveStruct {
    pub fn derive(s: Structure<'_>, data: &DataStruct, name: &Ident, attrs: &Vec<Attribute>, generics: &Generics) -> TokenStream {

        let tag = extract_tag(name, attrs);

        let mut state = Self {
            decode_fields: TokenStream::new(),
            decode_result: TokenStream::new(),
            encode_fields: TokenStream::new(),
        };

        for field in &data.fields {
            state.derive_field(field);
        }

        state.finish(&s, tag, generics)
    }

    /// Derive handling for a particular `#[field(...)]`
    fn derive_field(&mut self, field: &Field) {
        let attrs = FieldAttrs::new(field);
        self.derive_field_decoder(&attrs);
        self.derive_field_encoder(&attrs);
    }

    /// Derive code for decoding a field of a message
    fn derive_field_decoder(&mut self, field: &FieldAttrs) {
        let field_name = &field.name;
        let field_tag = field.tag;
        let field_decoder = quote! { let #field_name = decoder.decode_tagged_value(::simple_tlv::Tag::try_from(#field_tag).unwrap())?; };
        field_decoder.to_tokens(&mut self.decode_fields);

        let field_result = quote!(#field_name,);
        field_result.to_tokens(&mut self.decode_result);
    }

    /// Derive code for encoding a field of a message
    fn derive_field_encoder(&mut self, field: &FieldAttrs) {
        let field_name = &field.name;
        let field_tag = field.tag;
        let field_encoder = quote! { &(::simple_tlv::Tag::try_from(#field_tag).unwrap().with_value(&self.#field_name)), };
        field_encoder.to_tokens(&mut self.encode_fields);
    }

    /// Finish deriving a struct
    fn finish(self, s: &Structure<'_>, tag: u8, generics: &Generics) -> TokenStream {

        let lifetime = match parse_lifetime(generics) {
            Some(lifetime) => quote!(#lifetime),
            None => quote!('_),
        };

        let decode_fields = self.decode_fields;
        let decode_result = self.decode_result;
        let encode_fields = self.encode_fields;

        s.gen_impl(quote! {
            gen impl simple_tlv::Tagged for @Self {
                fn tag() -> simple_tlv::Tag {
                    // TODO(nickray): FIXME FIXME
                    use core::convert::TryFrom;
                    simple_tlv::Tag::try_from(#tag).unwrap()
                }
            }

            gen impl simple_tlv::Container for @Self {
                fn fields<F, T>(&self, field_encoder: F) -> simple_tlv::Result<T>
                where
                    F: FnOnce(&[&dyn simple_tlv::Encodable]) -> simple_tlv::Result<T>,
                {
                    use core::convert::TryFrom;
                    field_encoder(&[#encode_fields])
                }
            }
            gen impl<'a> core::convert::TryFrom<simple_tlv::TaggedSlice<'a>> for @Self {
                type Error = simple_tlv::Error;

                fn try_from(tagged_slice: simple_tlv::TaggedSlice<'a>) -> simple_tlv::Result<S> {
                    use core::convert::TryInto;
                    tagged_slice.tag().assert_eq(simple_tlv::Tag::try_from(#tag).unwrap())?;
                    tagged_slice.decode_nested(|decoder| {
                        #decode_fields

                        Ok(Self { #decode_result })
                    })
                }
            }
        })
    }
}

/// Parse the first lifetime of the "self" type of the custom derive
///
/// Returns `None` if there is no first lifetime.
fn parse_lifetime(generics: &Generics) -> Option<&Lifetime> {
    generics
        .lifetimes()
        .next()
        .map(|ref lt_ref| &lt_ref.lifetime)
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

// /// SIMPLE-TLV tags supported by the `#[tlv(tag = "...")]` attribute
// #[derive(Copy, Clone, Debug, Eq, PartialEq)]
// #[allow(clippy::enum_variant_names)]
// struct Tag(u8);
