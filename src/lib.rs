extern crate proc_macro;

use proc_macro::{TokenStream, TokenTree, Delimiter};
use quote::quote;
use syn::{Type, Expr, parse_macro_input};

#[proc_macro]
pub fn json(json: TokenStream) -> TokenStream {
	let mut iter = json.into_iter().peekable();
	let ty = iter.by_ref().take_while(|tt| if let TokenTree::Punct(punct) = tt { punct != &',' } else { true }).collect();
	let ty = parse_macro_input!(ty);
	json_impl(&ty, Box::new(iter))
}

fn json_impl<'a>(ty: &Type, iter: Box<dyn Iterator<Item = TokenTree> + 'a>) -> TokenStream {
	let mut iter = iter.peekable();
	if let Some(TokenTree::Group(group)) = iter.peek() {
		match group.delimiter() {
			Delimiter::Brace => {
				let mut iter = group.stream().into_iter().peekable();
				let mut entries = Vec::new();
				while iter.peek().is_some() {
					let key = iter.by_ref()
						.take_while(|tt| if let TokenTree::Punct(punct) = tt { punct != &':' } else { true }).collect();
					let key = parse_macro_input!(key as Expr);
					let value: proc_macro2::TokenStream = json_impl(ty, Box::new(iter.by_ref()
						.take_while(|tt| if let TokenTree::Punct(punct) = tt { punct != &',' } else { true }))).into();
					entries.push(quote!{ object.insert((#key).to_string(), #value); });
				}
				return (quote! {{
					let mut object = <#ty>::empty_object();
					#(#entries)*
					object.into()
				}}).into();
			},
			Delimiter::Bracket => {
				let mut iter = group.stream().into_iter().peekable();
				let mut values = Vec::<proc_macro2::TokenStream>::new();
				while iter.peek().is_some() {
					values.push(json_impl(ty, Box::new(iter.by_ref()
						.take_while(|tt| if let TokenTree::Punct(punct) = tt { punct != &',' } else { true }))).into());
				}
				return (quote! {{
					let mut array = <#ty>::empty_array();
					#(array.push_back(#values);)*
					array.into()
				}}).into();
			},
			_ => {}
		}
	}
	let expr = iter.collect();
	let expr = parse_macro_input!(expr as Expr);
	(quote! { <_ as Into<#ty>>::into(#expr) }).into()
}