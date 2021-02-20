use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Attribute, DataStruct, Field, Ident};
use synstructure::Structure;

use crate::{extract_attrs_optional_tag, FieldAttrs, Tag};

/// Derive Decodable on a struct
pub(crate) struct DeriveDecodableStruct {
    /// Field decoders
    decode_fields: TokenStream,

    /// Bound fields of a struct to be returned
    decode_result: TokenStream,
}

impl DeriveDecodableStruct {
    pub fn derive(s: Structure<'_>, data: &DataStruct, name: &Ident, attrs: &[Attribute]) -> TokenStream {

        let (tag, _slice) = extract_attrs_optional_tag(name, attrs);

        let mut state = Self {
            decode_fields: TokenStream::new(),
            decode_result: TokenStream::new(),
        };

        for field in &data.fields {
            state.derive_field(field);
        }

        state.finish(&s, tag)
    }

    /// Derive handling for a particular `#[field(...)]`
    fn derive_field(&mut self, field: &Field) {
        let attrs = FieldAttrs::new(field);
        self.derive_field_decoder(&attrs);
    }

    /// Derive code for decoding a field of a message
    fn derive_field_decoder(&mut self, field: &FieldAttrs) {
        let field_name = &field.name;
        let tag = field.tag;

        let field_decoder = match tag {
            Tag::Ber(tag) => {
                let class = tag.class as u8;
                let constructed = tag.constructed;
                let tag_number = tag.number;

                if field.slice {
                    quote! {
                        let tag = ::flexiber::Tag::from(
                            flexiber::Class::try_from(#class).unwrap(),
                            #constructed,
                            #tag_number
                        );
                        let #field_name =
                            decoder.decode_tagged_slice(tag)?.try_into().unwrap();
                                // .map_err(|_| flexiber::ErrorKind::Length { tag })?;
                    }
                } else {
                    quote! {
                        let tag = ::flexiber::Tag::from(
                            flexiber::Class::try_from(#class).unwrap(),
                            #constructed,
                            #tag_number
                        );

                        let #field_name = decoder.decode_tagged_value(tag)?;
                    }
                }
            }
            Tag::Simple(tag) => {
                let field_tag = tag.0;
                if field.slice {
                    quote! { let #field_name =
                        decoder.decode_tagged_slice(::flexiber::SimpleTag::try_from(#field_tag).unwrap())?.try_into()
                            .map_err(|_| {
                                use flexiber::TagLike;
                                flexiber::ErrorKind::Length { tag: flexiber::SimpleTag::try_from(#field_tag).unwrap().embedding() }
                            })?;
                    }
                } else {
                    quote! { let #field_name = decoder.decode_tagged_value(::flexiber::SimpleTag::try_from(#field_tag).unwrap())?; }
                }
            }
        };
        field_decoder.to_tokens(&mut self.decode_fields);

        let field_result = quote!(#field_name,);
        field_result.to_tokens(&mut self.decode_result);
    }

    /// Finish deriving a struct
    fn finish(self, s: &Structure<'_>, tag: Option<Tag>) -> TokenStream {

        let decode_fields = self.decode_fields;
        let decode_result = self.decode_result;

        if let Some(tag) = tag {

            match tag {
                Tag::Ber(tag) => {
                    let class = tag.class as u8;
                    let constructed = tag.constructed;
                    let tag_number = tag.number;

                    s.gen_impl(quote! {
                        gen impl<'a> core::convert::TryFrom<flexiber::TaggedSlice<'a>> for @Self {
                            type Error = flexiber::Error;

                            fn try_from(tagged_slice: flexiber::TaggedSlice<'a>) -> flexiber::Result<Self> {
                                use core::convert::TryInto;
                                use flexiber::TagLike;
                                let tag = ::flexiber::Tag::from(
                                    flexiber::Class::try_from(#class).unwrap(),
                                    #constructed,
                                    #tag_number
                                );
                                tagged_slice.tag().assert_eq(tag)?;
                                tagged_slice.decode_nested(|decoder| {
                                    #decode_fields

                                    Ok(Self { #decode_result })
                                })
                            }
                        }
                    })
                }
                Tag::Simple(tag) => {
                    let tag = tag.0;
                    s.gen_impl(quote! {
                        gen impl<'a> Decodable<'a> for @Self {
                            fn decode(decoder: &mut Decoder<'a>) -> Result<Self> {
                                flexiber::TaggedSlice::<'a, flexiber::SimpleTag>::decode(decoder)
                                    .and_then(|tagged_slice| {
                                        use core::convert::TryInto;
                                        use flexiber::TagLike;
                                        tagged_slice.tag().assert_eq(flexiber::SimpleTag::try_from(#tag).unwrap())?;
                                        tagged_slice.decode_nested(|decoder| {
                                            #decode_fields

                                            Ok(Self { #decode_result })
                                        })
                                    })
                                    .or_else(|e| decoder.error(e.kind()))
                            }

                        }
                    })
                }
            }
        } else {
            s.gen_impl(quote! {
                gen impl<'a> flexiber::Decodable<'a> for @Self {
                    fn decode(decoder: &mut flexiber::Decoder<'a>) -> flexiber::Result<Self> {
                        use core::convert::{TryFrom, TryInto};
                        #decode_fields
                        Ok(Self { #decode_result })
                    }
                }
            })
        }
    }
}

