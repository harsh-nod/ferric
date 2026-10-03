use syn::parse::{Parse, ParseStream};
use syn::{Ident, Item, Token};

fn tree(input: ParseStream<'_>) -> syn::Result<String> {
    input.step(|cursor| {
        let (tree, next) = cursor
            .token_tree()
            .ok_or_else(|| cursor.error("missing token tree"))?;
        Ok((tree.to_string(), next))
    })
}

fn ungroup(text: &str, left: char, right: char) -> &str {
    text.strip_prefix(left)
        .unwrap()
        .strip_suffix(right)
        .unwrap()
}

struct KernelParts {
    header: String,
    prelude: String,
    suffix: String,
}

impl Parse for KernelParts {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Ok(Self {
            header: ungroup(&tree(input)?, '[', ']').to_owned(),
            prelude: ungroup(&tree(input)?, '[', ']').to_owned(),
            suffix: ungroup(&tree(input)?, '[', ']').to_owned(),
        })
    }
}

struct OnlineArgs {
    parts: KernelParts,
    context: Ident,
    position: Ident,
    token: Ident,
    loader: String,
    math: Ident,
}

impl Parse for OnlineArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let emit: Ident = input.parse()?;
        assert_eq!(emit, "qwen_attention_emit_kernel_v1");
        input.parse::<Token![,]>()?;
        let parts = syn::parse_str(ungroup(&tree(input)?, '(', ')'))?;
        input.parse::<Token![,]>()?;
        let context = input.parse()?;
        input.parse::<Token![,]>()?;
        let position = input.parse()?;
        input.parse::<Token![,]>()?;
        let token = input.parse()?;
        input.parse::<Token![,]>()?;
        let loader = tree(input)?;
        syn::parse_str::<syn::Block>(&loader)?;
        input.parse::<Token![,]>()?;
        let math = input.parse()?;
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
        Ok(Self {
            parts,
            context,
            position,
            token,
            loader,
            math,
        })
    }
}

struct CallbackBody(String);

impl Parse for CallbackBody {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        input.parse::<Token![$]>()?;
        assert_eq!(input.parse::<Ident>()?, "state");
        Ok(Self(tree(input)?))
    }
}

struct Definition(String);

impl Parse for Definition {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let signature = tree(input)?;
        assert_eq!(
            signature.replace(' ', ""),
            "($emit:ident,$state:tt,$context:expr,$position:expr,$token:ident,$load:block,$math:ident)"
        );
        input.parse::<Token![=>]>()?;
        let body = tree(input)?;
        input.parse::<Token![;]>()?;
        // Substitute only the callback metavariable so syn can parse its call.
        let callback: syn::Macro =
            syn::parse_str(&ungroup(&body, '{', '}').replace("$ emit", "emit")).unwrap();
        assert!(callback.path.is_ident("emit"));
        Ok(Self(callback.parse_body::<CallbackBody>()?.0))
    }
}

struct NoExpressionMacros;

impl<'ast> syn::visit::Visit<'ast> for NoExpressionMacros {
    fn visit_macro(&mut self, _: &'ast syn::Macro) {
        panic!("kernel control flow must be visible before attribute expansion");
    }
}

// Expand only the three closed, audited macro definitions used by this kernel.
pub fn expand(wrapper: &str, online: &str) -> syn::File {
    let definition = syn::parse_file(online).unwrap();
    assert_eq!(definition.items.len(), 3);
    let callbacks = syn::parse_file(
        r#"
macro_rules! qwen_attention_emit_kernel_v1 {
    (([$($header:tt)*] [$($prelude:tt)*] [$($suffix:tt)*]) { $($body:tt)* }) => {
        $($header)* { $($prelude)* { $($body)* }; $($suffix)* }
    };
}
macro_rules! qwen_attention_emit_pair_v1 {
    (() { $($body:tt)* }) => { { $($body)* } };
}
"#,
    )
    .unwrap();
    assert_eq!(definition.items[..2], callbacks.items);
    let Item::Macro(item) = &definition.items[2] else {
        panic!("not macro definition")
    };
    assert!(item.attrs.is_empty());
    assert!(item.mac.path.is_ident("macro_rules"));
    assert_eq!(
        item.ident.as_ref().unwrap(),
        "qwen_attention_online_pair_v1"
    );
    let body = item.mac.parse_body::<Definition>().unwrap().0;
    let mut file = syn::parse_file(wrapper).unwrap();
    let modules: Vec<_> = file
        .items
        .iter()
        .filter(|item| matches!(item, Item::Mod(_)))
        .collect();
    assert_eq!(modules.len(), 1);
    let Item::Mod(module) = modules[0] else {
        unreachable!()
    };
    assert_eq!(module.ident, "online");
    assert_eq!(module.attrs.len(), 2);
    assert!(module.attrs[0].path().is_ident("macro_use"));
    assert_eq!(
        module.attrs[1],
        syn::parse_quote!(#[path = "attention_online.rs"])
    );
    assert!(module.content.is_none());
    file.items.retain(|item| !matches!(item, Item::Mod(_)));
    let mut count = 0;
    for item in &mut file.items {
        if let Item::Macro(invocation) = item {
            assert!(invocation.attrs.is_empty());
            assert!(invocation.ident.is_none());
            assert!(
                invocation
                    .mac
                    .path
                    .is_ident("qwen_attention_online_pair_v1")
            );
            let args: OnlineArgs = invocation.mac.parse_body().unwrap();
            let mut expanded = body.clone();
            for (name, replacement) in [
                ("$ context", args.context.to_string()),
                ("$ position", args.position.to_string()),
                ("$ token", args.token.to_string()),
                ("$ math", args.math.to_string()),
                ("$ load", args.loader),
            ] {
                expanded = expanded.replace(name, &replacement);
            }
            let root: syn::ItemFn = syn::parse_str(&format!(
                "{} {{ {} {}; {} }}",
                args.parts.header, args.parts.prelude, expanded, args.parts.suffix
            ))
            .unwrap();
            syn::visit::Visit::visit_item_fn(&mut NoExpressionMacros, &root);
            *item = Item::Fn(root);
            count += 1;
        }
    }
    assert_eq!(count, 1);
    file
}
