
use proc_macro::{TokenStream};

use proc_macro2::TokenStream as TokenStream2;

use quote::{quote, quote_spanned};

use syn::{Attribute, Expr, Field, Ident, Lifetime, Token, parse_macro_input, punctuated::Punctuated, spanned::Spanned, visit_mut::VisitMut};

mod parser;

#[proc_macro]
pub fn flat_serialize(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as FlatSerialize);
    let expanded = match input {
        FlatSerialize::Struct(input) => flat_serialize_struct(input),
        FlatSerialize::Enum(input) => flat_serialize_enum(input),
    };
    if cfg!(feature = "print-generated") {
        println!("{}", expanded.to_string());
    }
    expanded.into()
}

enum FlatSerialize {
    Enum(FlatSerializeEnum),
    Struct(FlatSerializeStruct),
}

/// a `flat_serialize`d enum e.g.
/// ```skip
/// flat_serialize! {
///     enum BasicEnum {
///         k: u8,
///         First: 2 {
///             data_len: usize,
///             data: [u8; self.data_len],
///         },
///         Fixed: 3 {
///             array: [u16; 3],
///         },
///     }
/// }
/// ```
/// the body of the enum variants must be the a valid FlatSerializeStruct body
struct FlatSerializeEnum {
    per_field_attrs: Vec<PerFieldsAttr>,
    attrs: Vec<Attribute>,
    ident: Ident,
    lifetime: Option<Lifetime>,
    tag: FlatSerializeField,
    variants: Punctuated<FlatSerializeVariant, Token![,]>,
}

struct FlatSerializeVariant {
    tag_val: Expr,
    body: FlatSerializeStruct,
}

/// a `flat_serialize`d struct e.g.
/// ```skip
/// flat_serialize! {
///     #[derive(Debug)]
///     struct Basic {
///         header: u64,
///         data_len: u32,
///         array: [u16; 3],
///         data: [u8; self.data_len],
///         data2: [u8; self.data_len / 2],
///     }
/// }
/// ```
/// the syntax is the same as a regular struct, except that it allows
/// `self` expressions in the length of arrays; these will be represented as
/// variable-length fields. We also interpret
/// `#[flat_serialize::field_attr(fixed = "#[foo]", variable = "#[bar]"))]` as
/// applying the attribute `#[foo]` to every fixed-length field of the struct,
/// and `#[bar]` to every variable-length field. e.g.
/// ```skip
/// flat_serialize! {
///     #[flat_serialize::field_attr(fixed = "#[foo]", variable = "#[bar]"))]`
///     struct Struct {
///         a: i32,
///         b: i32,
///         c: [u16; self.a]
///         d: [u8; self.a]
///     }
/// ```
/// is equivalent to
/// ```skip
/// flat_serialize! {
///     struct Struct {
///         #[foo]
///         a: i32,
///         #[foo]
///         b: i32,
///         #[bar]
///         c: [u16; self.a]
///         #[bar]
///         d: [u8; self.a]
///     }
/// ```
/// This can be useful when generating flat_serialize structs from a macro
struct FlatSerializeStruct {
    per_field_attrs: Vec<PerFieldsAttr>,
    attrs: Vec<Attribute>,
    ident: Ident,
    lifetime: Option<Lifetime>,
    fields: Punctuated<FlatSerializeField, Token![,]>,
}
struct FlatSerializeField {
    field: Field,
    ty_without_lifetime: Option<TokenStream2>,
    // TODO is this mutually exclusive with `flatten` above? Should we make an
    // enum to select between them?
    length_info: Option<VariableLenFieldInfo>,
}

/// a `#[flat_serialize::field_attr(fixed = "#[foo]", variable = "#[bar]"))]`
/// attribute. The inner attribute(s) will be applied to each relevant field.
struct PerFieldsAttr {
    fixed: Attribute,
    variable: Option<Attribute>,
}

/// how to find the length of a variable-length or optional field.
struct VariableLenFieldInfo {
    ty: syn::Type,
    ty_without_lifetime: Option<TokenStream2>,
    len_expr: syn::Expr,
    // is an optional field instead of a general varlen field, len_expr should
    // eval to a boolean
    is_optional: bool,
}

fn flat_serialize_struct(input: FlatSerializeStruct) -> TokenStream2 {
    let ident = input.ident.clone();

    let ref_def = {
        let alignment_check = input.alignment_check(quote!(0), quote!(8));
        let trait_check = input.fn_trait_check();
        let required_alignment = input.fn_required_alignment();
        let max_provided_alignment = input.fn_max_provided_alignment();
        let min_len = input.fn_min_len();

        // if we ever want to force #[repr(C)] we can use this code to derive
        // TRIVIAL_COPY from the struct fields
        let _const_len = input.fields.iter().map(|f| {
            if f.length_info.is_some() {
                quote!(false)
            } else {
                let ty = &f.ty;
                quote!( <#ty as flat_serialize::FlatSerializable>::TRIVIAL_COPY )
            }
        });

        let lifetime = input.lifetime.as_ref().map(|lifetime| {
            quote!{ #lifetime }
        });
        let try_ref = input.fn_try_ref(lifetime.as_ref());
        let fill_slice = input.fn_fill_slice();
        let len = input.fn_len();
        let fields = input
            .fields
            .iter()
            .map(|f| f.declaration(true, lifetime.as_ref(), input.per_field_attrs.iter()));
        let lifetime_args = input.lifetime.as_ref().map(|lifetime| {
            quote!{ <#lifetime> }
        });
        let ref_liftime = lifetime_args.clone().unwrap_or_else(|| quote!{ <'a> });
        let rl = lifetime.clone().unwrap_or_else(|| quote!{ 'a });

        let attrs = &*input.attrs;

        quote! {
            #[derive(Clone)]
            #(#attrs)*
            pub struct #ident #lifetime_args {
                #(#fields)*
            }

            // alignment assertions
            #[allow(unused_assignments)]
            const _: () = #alignment_check;

            #trait_check


            unsafe impl #ref_liftime flat_serialize::FlatSerializable #ref_liftime for #ident #lifetime_args {
                #required_alignment

                #max_provided_alignment

                #min_len

                // cannot be TRIVIAL_COPY unless the struct is #[repr(C)]
                const TRIVIAL_COPY: bool = false;
                type SLICE = flat_serialize::Iterable<#rl, #ident #lifetime_args>;

                #try_ref

                #fill_slice

                #len
            }
        }
    };

    let expanded = quote! {
        #ref_def
    };

    expanded
}

fn flat_serialize_enum(input: FlatSerializeEnum) -> TokenStream2 {
    let alignment_check = input.alignment_check();
    let uniqueness_check = input.uniqueness_check();
    let trait_check = input.fn_trait_check();
    let required_alignment = input.fn_required_alignment();
    let max_provided_alignment = input.fn_max_provided_alignment();
    let min_len = input.fn_min_len();

    let lifetime = input.lifetime.as_ref().map(|lifetime| quote!{ #lifetime });
    let lifetime_args = input.lifetime.as_ref().map(|lifetime| quote!{ <#lifetime> });
    let ref_liftime = lifetime_args.clone().unwrap_or_else(|| quote!{ <'a> });
    let rl = lifetime.clone().unwrap_or_else(|| quote!{ 'a });

    let try_ref = input.fn_try_ref(lifetime.as_ref());
    let fill_slice = input.fn_fill_slice();
    let len = input.fn_len();
    let body = input.variants(lifetime.as_ref());
    let ident = &input.ident;
    let attrs = &*input.attrs;



    quote! {
        #[derive(Clone)]
        #(#attrs)*
        #body

        #alignment_check

        #uniqueness_check

        #trait_check

        unsafe impl #ref_liftime flat_serialize::FlatSerializable #ref_liftime for #ident #lifetime_args {
            #required_alignment

            #max_provided_alignment

            #min_len

            // cannot be TRIVIAL_COPY since the rust enum layout is unspecified
            const TRIVIAL_COPY: bool = false;
            type SLICE = flat_serialize::Iterable<#rl, #ident #lifetime_args>;

            #try_ref

            #fill_slice

            #len
        }
    }
}

impl VariableLenFieldInfo {
    fn len_from_bytes(&self) -> TokenStream2 {
        let mut lfb = SelfReplacer(|name|
            syn::parse_quote! { #name.clone().unwrap() }
        );
        let mut len = self.len_expr.clone();
        lfb.visit_expr_mut(&mut len);
        quote! { #len }
    }

    fn counter_expr(&self) -> TokenStream2 {
        let mut ce = SelfReplacer(|name|
            syn::parse_quote! { (*#name) }
        );
        let mut len = self.len_expr.clone();
        ce.visit_expr_mut(&mut len);
        quote! { #len }
    }

    fn err_size_expr(&self) -> TokenStream2 {
        let mut ese = SelfReplacer(|name|
            syn::parse_quote! {
                match #name { Some(#name) => #name, None => return 0usize, }
            }
        );
        let mut len = self.len_expr.clone();
        ese.visit_expr_mut(&mut len);
        quote! { #len }
    }
}

struct SelfReplacer<F: FnMut(&Ident) -> syn::Expr>(F);

impl<F: FnMut(&Ident) -> syn::Expr> VisitMut for SelfReplacer<F> {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        if let syn::Expr::Field(field) = expr {
            if let syn::Expr::Path(path) = &mut *field.base {
                if path.path.segments[0].ident == "self" {
                    let name = match &field.member {
                        syn::Member::Named(name) => name,
                        syn::Member::Unnamed(_) => panic!("unnamed fields not supported"),
                    };
                    *expr = self.0(name)
                }
            }
        } else {
            syn::visit_mut::visit_expr_mut(self, expr)
        }
    }
}

struct TryRefBody {
    vars: TokenStream2,
    body: TokenStream2,
    set_fields: TokenStream2,
    err_size: TokenStream2,
}

impl FlatSerializeEnum {
    fn variants(&self, lifetime: Option<&TokenStream2>) -> TokenStream2 {
        let id = &self.ident;
        let variants = self.variants.iter().map(|variant| {
            let fields = variant.body.fields.iter().map(|f|
                f.declaration(false, lifetime, self.per_field_attrs.iter())
            );
            let ident = &variant.body.ident;
            quote! {
                #ident {
                    #(#fields)*
                },
            }
        });
        let args = lifetime.map(|lifetime| quote!{ <#lifetime> });
        quote! {
            pub enum #id #args {
                #(#variants)*
            }
        }
    }

    fn uniqueness_check(&self) -> TokenStream2 {
        let variants = self.variants.iter().map(|variant| {
            let ident = &variant.body.ident;
            let tag_val = &variant.tag_val;
            quote! {
                #ident = #tag_val,
            }
        });
        quote! {
            // uniqueness check
            const _: () = {
                #[allow(dead_code)]
                enum UniquenessCheck {
                    #(#variants)*
                }
            };
        }
    }

    fn alignment_check(&self) -> TokenStream2 {
        let tag_check = self.tag.alignment_check();
        let variant_checks = self.variants.iter()
            .map(|v| v.body.alignment_check(quote!(current_size), quote!(min_align)));
        quote! {
            // alignment assertions
            #[allow(unused_assignments)]
            const _: () = {
                use std::mem::{align_of, size_of};
                let mut current_size = 0;
                let mut min_align = 8;
                #tag_check
                #(#variant_checks)*
            };
        }
    }

    fn fn_trait_check(&self) -> TokenStream2 {
        let tag_check = self.tag.trait_check();
        let checks = self.variants.iter().map(|v| v.body.fn_trait_check());
        quote! {
            const _: () = {
                #tag_check
                #(
                    const _: () = {
                        #checks
                    };
                )*
            };
        }
    }

    fn fn_required_alignment(&self) -> TokenStream2 {
        let tag_alignment = self.tag.required_alignment();
        let alignments = self.variants.iter().map(|v| {
            let alignments = v.body.fields.iter().map(|f| f.required_alignment());
            quote!{
                let mut required_alignment = #tag_alignment;
                #(
                    let alignment = #alignments;
                    if alignment > required_alignment {
                        required_alignment = alignment;
                    }
                )*
                required_alignment
            }
        });

        quote! {
            const REQUIRED_ALIGNMENT: usize = {
                use std::mem::align_of;
                let mut required_alignment: usize = #tag_alignment;
                #(
                    let alignment: usize = {
                        #alignments
                    };
                    if alignment > required_alignment {
                        required_alignment = alignment;
                    }
                )*
                required_alignment
            };
        }
    }

    fn fn_max_provided_alignment(&self) -> TokenStream2 {
        let min_align = self.tag.max_provided_alignment();
        let min_align = quote!{
            match #min_align {
                Some(a) => Some(a),
                None => Some(8),
            }
        };

        let min_size = self.tag.min_len();

        let alignments = self.variants.iter().map(|v| {
            let alignments = v.body.fields.iter().map(|f| f.max_provided_alignment());
            let sizes = v.body.fields.iter().map(|f| f.min_len());
            quote!{
                let mut min_align: Option<usize> = #min_align;
                #(
                    let alignment = #alignments;
                    match (alignment, min_align) {
                        (None, _) => (),
                        (Some(align), None) => min_align = Some(align),
                        (Some(align), Some(min)) if align < min =>
                            min_align = Some(align),
                        _ => (),
                    }
                )*
                let variant_size: usize = #min_size #(+ #sizes)*;
                let effective_alignment = match min_align {
                    Some(align) => align,
                    None => 8,
                };

                if variant_size % 8 == 0 && effective_alignment >= 8 {
                    8
                } else if variant_size % 4 == 0 && effective_alignment >= 4 {
                    4
                } else if variant_size % 2 == 0 && effective_alignment >= 2 {
                    2
                } else {
                    1
                }
            }
        });
        quote! {
            const MAX_PROVIDED_ALIGNMENT: Option<usize> = {
                use std::mem::{align_of, size_of};
                let mut min_align: usize = match #min_align {
                    None => 8,
                    Some(align) => align,
                };
                #(
                    let variant_alignment: usize = {
                        #alignments
                    };
                    if variant_alignment < min_align {
                        min_align = variant_alignment
                    }
                )*
                let min_size = Self::MIN_LEN;
                if min_size % 8 == 0 && min_align >= 8 {
                    Some(8)
                } else if min_size % 4 == 0 && min_align >= 4 {
                    Some(4)
                } else if min_size % 2 == 0 && min_align >= 2 {
                    Some(2)
                } else {
                    Some(1)
                }
            };
        }
    }

    fn fn_min_len(&self) -> TokenStream2 {
        let tag_size = self.tag.min_len();
        let sizes = self.variants.iter().map(|v| {
            let sizes = v.body.fields.iter().map(|f| f.min_len());
            quote! {
                let mut size: usize = #tag_size;
                #(size += #sizes;)*
                size
            }
        });
        quote! {
            const MIN_LEN: usize = {
                use std::mem::size_of;
                let mut size: Option<usize> = None;
                #(
                    let variant_size = {
                        #sizes
                    };
                    size = match size {
                        None => Some(variant_size),
                        Some(size) if size > variant_size => Some(variant_size),
                        Some(size) => Some(size),
                    };
                )*
                match size {
                    Some(size) => size,
                    None => #tag_size,
                }
            };
        }
    }

    fn fn_try_ref(&self, lifetime: Option<&TokenStream2>) -> TokenStream2 {
        let break_label = syn::Lifetime::new("'tryref_tag", proc_macro2::Span::call_site());
        let try_wrap_tag = self.tag.try_wrap(&break_label);
        let id = &self.ident;
        let tag_ty = &self.tag.ty;

        let bodies = self.variants.iter().enumerate().map(|(i, v)| {
            let tag_val = &v.tag_val;

            let variant = &v.body.ident;

            let break_label =
                syn::Lifetime::new(&format!("'tryref_{}", i), proc_macro2::Span::call_site());

            let TryRefBody {
                vars,
                body,
                set_fields,
                err_size,
            } = v
                .body
                .fn_try_ref_body(&break_label);

            quote! {
                Some(#tag_val) => {
                    #vars
                    #break_label: loop {
                        #body
                        let _ref = #id::#variant { #set_fields };
                        return Ok((_ref, input))
                    }
                    return Err(flat_serialize::WrapErr::NotEnoughBytes(std::mem::size_of::<#tag_ty>() #err_size))
                }
            }
        });

        let tag_ident = self.tag.ident.as_ref().unwrap();

        quote! {
            #[allow(unused_assignments, unused_variables)]
            #[inline(always)]
            unsafe fn try_ref(mut input: & #lifetime [u8]) -> Result<(Self, & #lifetime [u8]), flat_serialize::WrapErr> {
                let __packet_macro_read_len = 0usize;
                let mut #tag_ident = None;
                'tryref_tag: loop {
                    #try_wrap_tag;
                    match #tag_ident {
                        #(#bodies),*
                        _ => return Err(flat_serialize::WrapErr::InvalidTag(0)),
                    }
                }
                //TODO
                Err(flat_serialize::WrapErr::NotEnoughBytes(::std::mem::size_of::<#tag_ty>()))
            }
        }
    }

    fn fn_fill_slice(&self) -> TokenStream2 {
        let tag_ty = &self.tag.ty;
        let tag_ident = self.tag.ident.as_ref().unwrap();
        let fill_slice_tag = self.tag.fill_slice();
        let id = &self.ident;
        let bodies = self.variants.iter().map(|v| {
            let tag_val = &v.tag_val;
            let variant = &v.body.ident;
            let (fields, fill_slice_with) = v.body.fill_slice_body();
            quote! {
                #id::#variant { #fields } => {
                    let #tag_ident: &#tag_ty = &#tag_val;
                    #fill_slice_tag
                    #fill_slice_with
                }
            }
        });
        quote! {
            #[allow(unused_assignments, unused_variables)]
            unsafe fn fill_slice<'out>(&self, input: &'out mut [std::mem::MaybeUninit<u8>])
            -> &'out mut [std::mem::MaybeUninit<u8>] {
                let total_len = self.len();
                let (mut input, rem) = input.split_at_mut(total_len);
                match self {
                    #(#bodies),*
                }
                debug_assert_eq!(input.len(), 0);
                rem
            }
        }
    }

    fn fn_len(&self) -> TokenStream2 {
        let tag_ty = &self.tag.ty;
        let tag_size = quote! { ::std::mem::size_of::<#tag_ty>() };
        let id = &self.ident;
        let bodies = self.variants.iter().map(|v| {
            let variant = &v.body.ident;

            let size = v
                .body
                .fields
                .iter()
                .map(|f| f.size_fn());
            let fields = v.body.fields.iter().map(|f| f.ident.as_ref().unwrap());
            quote! {
                #id::#variant { #(#fields),* } => {
                    #tag_size #(+ #size)*
                },
            }
        });
        quote! {
            #[allow(unused_assignments, unused_variables)]
            fn len(&self) -> usize {
                match self {
                    #(#bodies)*
                }
            }
        }
    }
}

impl FlatSerializeStruct {
    fn alignment_check(
        &self,
        start: TokenStream2,
        min_align: TokenStream2,
    ) -> TokenStream2 {
        let checks = self.fields.iter().map(|f| f.alignment_check());

        quote! {
            {
                use std::mem::{align_of, size_of};
                let mut current_size = #start;
                let mut min_align = #min_align;
                #(#checks)*
            }
        }
    }

    fn fn_trait_check(&self) -> TokenStream2 {
        let checks = self.fields.iter().map(|f| f.trait_check());
        quote! {
            const _: () = {
                #(#checks)*
            };
        }
    }

    fn fn_required_alignment(&self) -> TokenStream2 {
        let alignments = self.fields.iter().map(|f| f.required_alignment());
        quote! {
            const REQUIRED_ALIGNMENT: usize = {
                use std::mem::align_of;
                let mut required_alignment = 1;
                #(
                    let alignment = #alignments;
                    if alignment > required_alignment {
                        required_alignment = alignment;
                    }
                )*
                required_alignment
            };
        }
    }

    fn fn_max_provided_alignment(&self) -> TokenStream2 {
        let alignments = self.fields.iter().map(|f| f.max_provided_alignment());
        quote! {
            const MAX_PROVIDED_ALIGNMENT: Option<usize> = {
                use std::mem::align_of;
                let mut min_align: Option<usize> = None;
                #(
                    let ty_align = #alignments;
                    match (ty_align, min_align) {
                        (None, _) => (),
                        (Some(align), None) => min_align = Some(align),
                        (Some(align), Some(min)) if align < min =>
                            min_align = Some(align),
                        _ => (),
                    }
                )*
                match min_align {
                    None => None,
                    Some(min_align) => {
                        let min_size = Self::MIN_LEN;
                        if min_size % 8 == 0 && min_align >= 8 {
                            Some(8)
                        } else if min_size % 4 == 0 && min_align >= 4 {
                            Some(4)
                        } else if min_size % 2 == 0 && min_align >= 2 {
                            Some(2)
                        } else {
                            Some(1)
                        }
                    },
                }
            };
        }
    }

    fn fn_min_len(&self) -> TokenStream2 {
        let sizes = self.fields.iter().map(|f| f.min_len());
        quote! {
            const MIN_LEN: usize = {
                use std::mem::size_of;
                let mut size = 0;
                #(size += #sizes;)*
                size
            };
        }
    }

    fn fn_try_ref(
        &self,
        lifetime: Option<&TokenStream2>,
    ) -> TokenStream2 {
        let break_label = syn::Lifetime::new("'tryref", proc_macro2::Span::call_site());
        let id = &self.ident;
        let TryRefBody {
            vars,
            body,
            set_fields,
            err_size,
        } = self.fn_try_ref_body(&break_label);
        quote! {
            #[allow(unused_assignments, unused_variables)]
            #[inline(always)]
            unsafe fn try_ref(mut input: & #lifetime [u8])
            -> Result<(Self, & #lifetime [u8]), flat_serialize::WrapErr> {
                if input.len() < Self::MIN_LEN {
                    return Err(flat_serialize::WrapErr::NotEnoughBytes(Self::MIN_LEN))
                }
                let __packet_macro_read_len = 0usize;
                #vars
                #break_label: loop {
                    #body
                    let _ref = #id { #set_fields };
                    return Ok((_ref, input))
                }
                Err(flat_serialize::WrapErr::NotEnoughBytes(0 #err_size))
            }
        }
    }

    fn fn_try_ref_body(
        &self,
        break_label: &syn::Lifetime,
    ) -> TryRefBody {
        let field_names = self
            .fields
            .iter()
            .map(|f| &f.ident);
        let ty1 =  self
            .fields
            .iter()
            .map(|f| f.local_ty());
        let field1 = field_names.clone();
        let field2 = field_names.clone();
        let field_setters = self.fields.iter().map(|field| {
            let name = &field.ident;
            if field.is_optional() {
                quote!{ #name }
            } else {
                quote!{ #name.unwrap() }
            }
        });

        let vars = quote!( #(let mut #field1: #ty1 = None;)* );
        let try_wrap_fields = self.fields.iter().map(|f| f.try_wrap(break_label));
        let body = quote! ( #(#try_wrap_fields)* );

        let set_fields = quote!( #(#field2: #field_setters),* );

        let err_size = self
        .fields
            .iter()
            .map(|f| f.err_size());
        let err_size = quote!( #( + #err_size)* );
        TryRefBody {
            vars,
            body,
            set_fields,
            err_size,
        }
    }

    fn fn_fill_slice(
        &self,
    ) -> TokenStream2 {
        let id = &self.ident;
        let (fields, fill_slice_with) = self.fill_slice_body();
        quote! {
            #[allow(unused_assignments, unused_variables)]
            #[inline(always)]
            unsafe fn fill_slice<'out>(&self, input: &'out mut [std::mem::MaybeUninit<u8>]) -> &'out mut [std::mem::MaybeUninit<u8>] {
                let total_len = self.len();
                let (mut input, rem) = input.split_at_mut(total_len);
                let #id { #fields } = self;
                #fill_slice_with
                debug_assert_eq!(input.len(), 0);
                rem
            }
        }
    }
    fn fill_slice_body(
        &self,
    ) -> (TokenStream2, TokenStream2) {
        //FIXME assert multiple values of counters are equal...
        let fill_slice_with = self.fields.iter().map(|f| f.fill_slice());
        let fill_slice_with = quote!( #(#fill_slice_with);* );

        let field = self.fields.iter().map(|f| f.ident.as_ref().unwrap());
        let fields = quote!( #(#field),* );
        (fields, fill_slice_with)
    }

    fn fn_len(
        &self,
    ) -> TokenStream2 {
        let size = self
            .fields
            .iter()
            .map(|f| f.size_fn());
        let field = self.fields.iter().map(|f| f.ident.as_ref().unwrap());
        let id = &self.ident;

        quote! {
            #[allow(unused_assignments, unused_variables)]
            #[inline(always)]
            fn len(&self) -> usize {
                let #id { #(#field),* } = self;
                0usize #(+ #size)*
            }
        }
    }
}

impl FlatSerializeField {

    fn alignment_check(&self) -> TokenStream2 {
        let current_size  = quote!(current_size);
        let min_align  = quote!(min_align);
        match &self.length_info {
            None => {
                let ty = self.ty_without_lifetime();
                quote_spanned!{self.ty.span()=>
                    let _alignment_check: () = [()][(#current_size) % <#ty as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
                    let _alignment_check2: () = [()][(<#ty as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > #min_align) as u8 as usize];
                    #current_size += <#ty as flat_serialize::FlatSerializable>::MIN_LEN;
                    #min_align = match <#ty as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
                        Some(align) if align < #min_align => align,
                        _ => #min_align,
                    };
                }
            }
            Some(info) => {
                let ty = info.ty_without_lifetime();
                quote_spanned!{self.ty.span()=>
                    let _alignment_check: () = [()][(#current_size) % <#ty as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT];
                    let _alignment_check2: () = [()][(<#ty as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT > #min_align) as u8 as usize];
                    if <#ty as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT < #min_align {
                        #min_align = <#ty as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
                    }
                    #min_align = match <#ty as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT {
                        Some(align) if align < #min_align => align,
                        _ => #min_align,
                    };
                }
            }
        }
    }

    fn trait_check(&self) -> TokenStream2 {
        let (ty, needs_lifetime) =
            match (&self.ty_without_lifetime, &self.length_info) {
                (_, Some(VariableLenFieldInfo{ ty_without_lifetime: Some(ty), .. })) =>
                    (ty.clone(), true),
                (_, Some(VariableLenFieldInfo{ ty, .. })) => (quote!{ #ty }, false),
                (Some(ty), _) => (ty.clone(), true),
                _ => {
                    let ty = &self.ty;
                    (quote!{ #ty }, false)
                }
            };
        let lifetime = needs_lifetime.then(|| quote!{ <'static> });
        let name = self.ident.as_ref().unwrap();
        // based on static_assertions
        // TODO add ConstLen assertion if type is in var-len position?
        return quote_spanned!{self.ty.span()=>
            fn #name<'test, T: flat_serialize::FlatSerializable<'test>>() {}
            let _ = #name::<#ty #lifetime>;
        }
    }

    fn required_alignment(&self) -> TokenStream2 {
        let ty = match &self.length_info {
            None => self.ty_without_lifetime(),
            Some(info) => info.ty_without_lifetime(),
        };
        quote_spanned!{self.ty.span()=>
            <#ty as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT
        }
    }

    fn max_provided_alignment(&self) -> TokenStream2 {
        match &self.length_info {
            None => {
                let ty = self.ty_without_lifetime();
                quote_spanned!{self.ty.span()=>
                    <#ty as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT
                }
            },
            Some(info @ VariableLenFieldInfo { is_optional: true, ..}) => {
                let ty = info.ty_without_lifetime();
                // fields after an optional field cannot be aligned to more than
                // the field is in the event the field is present, so if the
                // field does not provide a max alignment (i.e. it's fixed-len)
                // use that to determine what the max alignment is.
                quote_spanned!{self.ty.span()=>
                    {
                        let ty_provied = <#ty as flat_serialize::FlatSerializable>::MAX_PROVIDED_ALIGNMENT;
                        match ty_provied {
                            Some(align) => Some(align),
                            None => Some(<#ty as flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT),
                        }
                    }
                }
            },
            Some(info @ VariableLenFieldInfo { is_optional: false, ..}) => {
                let ty = info.ty_without_lifetime();
                // for variable length slices we only need to check the required
                // alignment, not the max-provided: TRIVIAL_COPY types won't
                // have a max-provided alignment, while other ones will be
                // padded out to their natural alignment.
                quote_spanned!{self.ty.span()=>
                    {
                        Some(<#ty as  flat_serialize::FlatSerializable>::REQUIRED_ALIGNMENT)
                    }
                }
            }
        }
    }

    fn min_len(&self) -> TokenStream2 {
        match &self.length_info {
            None => {
                let ty = self.ty_without_lifetime();
                quote_spanned!{self.ty.span()=>
                    <#ty as flat_serialize::FlatSerializable>::MIN_LEN
                }
            },
            Some(..) =>
                quote_spanned!{self.ty.span()=>
                    0
                },
        }
    }

    fn try_wrap(&self, break_label: &syn::Lifetime,) -> TokenStream2 {
        let ident = self.ident.as_ref().unwrap();
        match &self.length_info {
            Some(info @ VariableLenFieldInfo { is_optional: false, .. }) => {
                let count = info.len_from_bytes();
                let ty = &info.ty;
                if parser::has_lifetime(ty) {
                    return quote_spanned!{ty.span()=>
                        compile_error!("flattened types are not allowed in variable-length fields")
                    }
                }
                quote! {
                    {
                        let count = (#count) as usize;
                        let (field, rem) = match <_ as flat_serialize::Slice <'_
                        >>::try_ref(input, count) {
                            Ok((f, b)) => (f, b),
                            Err(flat_serialize::WrapErr::InvalidTag(offset)) =>
                                return Err(flat_serialize::WrapErr::InvalidTag(__packet_macro_read_len + offset)),
                            Err(..) => break #break_label
                        };
                        input = rem;
                        #ident = Some(field);
                    }
                }
            }
            Some(info @ VariableLenFieldInfo { is_optional: true, .. }) => {
                let is_present = info.len_from_bytes();
                let ty = info.ty_without_lifetime();
                quote! {
                    if #is_present {
                        let (field, rem) = match <#ty>::try_ref(input) {
                            Ok((f, b)) => (f, b),
                            Err(flat_serialize::WrapErr::InvalidTag(offset)) =>
                                return Err(flat_serialize::WrapErr::InvalidTag(__packet_macro_read_len + offset)),
                            Err(..) => break #break_label

                        };
                        input = rem;
                        #ident = Some(field);
                    }
                }
            }
            None => {
                let ty = self.ty_without_lifetime();
                quote!{
                    {
                        let (field, rem) = match <#ty>::try_ref(input) {
                            Ok((f, b)) => (f, b),
                            Err(flat_serialize::WrapErr::InvalidTag(offset)) =>
                                return Err(flat_serialize::WrapErr::InvalidTag(__packet_macro_read_len + offset)),
                            Err(..) => break #break_label

                        };
                        input = rem;
                        #ident = Some(field);
                    }
                }
            }
        }
    }

    fn fill_slice(&self) -> TokenStream2 {
        let ident = self.ident.as_ref().unwrap();
        match &self.length_info {
            Some(info @ VariableLenFieldInfo { is_optional: false, .. }) => {
                let count = info.counter_expr();
                // TODO this may not elide all bounds checks
                quote! {
                    unsafe {
                        let count = (#count) as usize;
                        input = <_ as flat_serialize::Slice<'_>>::fill_slice(#ident, count, input);
                    }
                }
            }
            Some(info @ VariableLenFieldInfo { is_optional: true, .. }) => {
                let is_present = info.counter_expr();
                let ty = &info.ty;
                quote! {
                    unsafe {
                        if #is_present {
                            let #ident: &#ty = #ident.as_ref().unwrap();
                            input = #ident.fill_slice(input);
                        }
                    }
                }
            }
            None => {
                quote! {
                    unsafe {
                        input = #ident.fill_slice(input);
                    }
                }
            }
        }
    }

    fn err_size(&self) -> TokenStream2 {
        match &self.length_info {
            Some(info @ VariableLenFieldInfo { is_optional: false, .. }) => {
                let count = info.err_size_expr();
                let ty = info.ty_without_lifetime();
                quote! {
                    (|| <#ty>::MIN_LEN * (#count) as usize)()
                }
            }
            Some(info @ VariableLenFieldInfo { is_optional: true, .. }) => {
                let is_present = info.err_size_expr();
                let ty = info.ty_without_lifetime();
                quote! {
                    (|| if #is_present { <#ty>::MIN_LEN } else { 0 })()
                }
            }
            None => {
                let ty = &self.ty_without_lifetime();
                quote! { <#ty>::MIN_LEN }
            },
        }
    }

    fn exposed_ty(&self, lifetime: Option<&TokenStream2>) -> TokenStream2 {
        match &self.length_info {
            None => {
                let nominal_ty = &self.ty;
                quote_spanned! {self.field.span()=>
                    #nominal_ty
                }
            },
            Some(VariableLenFieldInfo { is_optional: false, ty, .. }) =>
                quote_spanned! {self.field.span()=>
                    <#ty as flat_serialize::FlatSerializable<#lifetime>>::SLICE
                },
            Some(VariableLenFieldInfo { is_optional: true, ty, .. }) => {
                quote_spanned! {self.field.span()=>
                    Option<#ty>
                }
            },
        }
    }

    fn local_ty(&self) -> TokenStream2 {
        match &self.length_info {
            None => {
                let ty = &self.ty;
                quote! { Option<#ty> }
            },
            Some(VariableLenFieldInfo { is_optional: false, ty, .. }) => {
                quote! { Option<<#ty as flat_serialize::FlatSerializable<'_>>::SLICE> }
            },
            Some(VariableLenFieldInfo { is_optional: true, ty, .. }) => {
                quote! { Option<#ty> }
            },
        }
    }

    fn size_fn(&self) -> TokenStream2 {
        let ident = self.ident.as_ref().unwrap();
        match &self.length_info {
            Some(info @ VariableLenFieldInfo { is_optional: false, .. }) => {
                let count = info.counter_expr();
                quote! {
                    (<_ as flat_serialize::Slice<'_>>::len(#ident, (#count) as usize))
                }
            }
            Some(info @ VariableLenFieldInfo { is_optional: true, .. }) => {
                let ty = self.ty_without_lifetime();
                let is_present = info.counter_expr();
                quote! {
                    (if #is_present {
                        <#ty as flat_serialize::FlatSerializable>::len(#ident.as_ref().unwrap())
                    } else {
                        0
                    })
                }
            }
            None => {
                let nominal_ty = self.ty_without_lifetime();
                quote!( <#nominal_ty as flat_serialize::FlatSerializable>::len(#ident) )
            }
        }
    }

    fn declaration<'a, 'b: 'a>(
        &'b self,
        is_pub: bool,
        lifetime: Option<&TokenStream2>,
        pf_attrs: impl Iterator<Item = &'a PerFieldsAttr> + 'a
    ) -> TokenStream2 {
        let name = self.ident.as_ref().unwrap();
        let attrs = self.attrs.iter();
        let pub_marker = is_pub.then(|| quote!{ pub });
        let ty = self.exposed_ty(lifetime);
        let per_field_attrs = self.per_field_attrs(pf_attrs);
        quote! { #(#per_field_attrs)* #(#attrs)* #pub_marker #name: #ty, }
    }

    fn per_field_attrs<'a, 'b: 'a>(
        &'b self,
        attrs: impl Iterator<Item = &'a PerFieldsAttr> + 'a,
    ) -> impl Iterator<Item = TokenStream2> + 'a {
        attrs.map(move |attr| match &self.length_info {
            None => {
                let attr = &attr.fixed;
                quote! { #attr }
            }
            Some(_) => match &attr.variable {
                Some(attr) => quote! { #attr },
                None => quote! {},
            },
        })
    }

    fn ty_without_lifetime(&self) -> TokenStream2 {
        match &self.ty_without_lifetime {
            None => {
                let ty = &self.ty;
                quote!{ #ty }
            },
            Some(ty) => ty.clone(),
        }
    }

    fn is_optional(&self) -> bool {
        matches!(self.length_info, Some(VariableLenFieldInfo { is_optional: true, ..}))
    }
}

impl VariableLenFieldInfo {
    fn ty_without_lifetime(&self) -> TokenStream2 {
        match &self.ty_without_lifetime {
            None => {
                let ty = &self.ty;
                quote!{ #ty }
            },
            Some(ty) => ty.clone(),
        }
    }
}

#[proc_macro_derive(FlatSerializable)]
pub fn flat_serializable_derive(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let name = input.ident;

    let s = match input.data {
        syn::Data::Enum(e) => {
            let repr: Vec<_> = input.attrs.iter().flat_map(|attr| {
                let meta = match attr.parse_meta() {
                    Ok(meta) => meta,
                    _ => return None,
                };
                let has_repr = meta.path().get_ident().map_or(false, |id| id == "repr");
                if !has_repr {
                    return None
                }
                attr.parse_args().ok().and_then(|ident: Ident| {
                    if ident == "u8" || ident == "u16" || ident == "u32" || ident == "u64" {
                        return Some(ident)
                    }
                    None
                })
            }).collect();
            if repr.len() != 1 {
                return quote_spanned! {e.enum_token.span()=>
                    compile_error!{"FlatSerializable only allowed on #[repr(u..)] enums without variants"}
                }.into()
            }
            let all_unit = e.variants.iter().all(|variant| matches!(variant.fields, syn::Fields::Unit));
            if !all_unit {
                return quote_spanned! {e.enum_token.span()=>
                    compile_error!{"FlatSerializable only allowed on until enums"}
                }.into()
            }

            let variant = e.variants.iter().map(|v| &v.ident);
            let variant2 = variant.clone();
            let const_name = variant.clone();
            let repr = &repr[0];

            let out = quote!{
                unsafe impl<'i> flat_serialize::FlatSerializable<'i> for #name {
                    const MIN_LEN: usize = std::mem::size_of::<Self>();
                    const REQUIRED_ALIGNMENT: usize = std::mem::align_of::<Self>();
                    const MAX_PROVIDED_ALIGNMENT: Option<usize> = None;
                    const TRIVIAL_COPY: bool = true;
                    type SLICE = &'i [#name];

                    #[inline(always)]
                    #[allow(non_upper_case_globals)]
                    unsafe fn try_ref(input: &'i [u8])
                    -> Result<(Self, &'i [u8]), flat_serialize::WrapErr> {
                        let size = std::mem::size_of::<Self>();
                        if input.len() < size {
                            return Err(flat_serialize::WrapErr::NotEnoughBytes(size))
                        }
                        let (field, rem) = input.split_at(size);
                        let field = field.as_ptr().cast::<#repr>();
                        #(
                            const #const_name: #repr = #name::#variant2 as #repr;
                        )*
                        let field = field.read_unaligned();
                        let field = match field {
                            #(#variant => #name::#variant,)*
                            _ => return Err(flat_serialize::WrapErr::InvalidTag(0)),
                        };
                        Ok((field, rem))
                    }

                    #[inline(always)]
                    unsafe fn fill_slice<'out>(&self, input: &'out mut [std::mem::MaybeUninit<u8>])
                    -> &'out mut [std::mem::MaybeUninit<u8>] {
                        let size = std::mem::size_of::<Self>();
                        let (input, rem) = input.split_at_mut(size);
                        let bytes = (self as *const Self).cast::<std::mem::MaybeUninit<u8>>();
                        let bytes = std::slice::from_raw_parts(bytes, size);
                        input.copy_from_slice(bytes);
                        rem
                    }

                    #[inline(always)]
                    fn len(&self) -> usize {
                        std::mem::size_of::<Self>()
                    }
                }
            };
            return out.into();
        },
        syn::Data::Union(u) => return quote_spanned! {u.union_token.span()=>
            compile_error!("FlatSerializable not allowed on unions")
        }.into(),
        syn::Data::Struct(s) => s,

    };

    let s = FlatSerializeStruct {
        per_field_attrs: Default::default(),
        attrs: Default::default(),
        ident: name.clone(),
        lifetime: None,
        fields: s.fields.into_iter().map(|f| FlatSerializeField {
            field: f,
            ty_without_lifetime: None,
            length_info: None,
        }).collect(),
    };

    let ident = &s.ident;
    let alignment_check = s.alignment_check(quote!(0), quote!(8));
    let trait_check = s.fn_trait_check();
    let required_alignment = s.fn_required_alignment();
    let max_provided_alignment = s.fn_max_provided_alignment();
    let min_len = s.fn_min_len();

    let try_ref = s.fn_try_ref(None);
    let fill_slice = s.fn_fill_slice();
    let len = s.fn_len();

    // FIXME add check that all values are TRIVIAL_COPY
    let out = quote! {

        // alignment assertions
        #[allow(unused_assignments)]
        const _: () = #alignment_check;

        #trait_check

        unsafe impl<'a> flat_serialize::FlatSerializable<'a> for #ident {
            #required_alignment

            #max_provided_alignment

            #min_len

            const TRIVIAL_COPY: bool = true;
            type SLICE = &'a [#ident];

            #try_ref

            #fill_slice

            #len
        }
    };
    out.into()
}
