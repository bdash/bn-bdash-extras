use syn::{
    Expr, Pat, Token,
    parse::{Parse, ParseStream, Result},
};

#[derive(Debug, Clone)]
pub struct MacroInput {
    pub expr: Expr,
    pub arms: Vec<MatchArm>,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pat,
    pub guard: Option<Expr>,
    pub body: Expr,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let expr: Expr = input.parse()?;
        input.parse::<Token![,]>()?;

        let mut arms = Vec::new();
        while !input.is_empty() {
            let arm: MatchArm = input.parse()?;
            let needs_comma = match &arm.body {
                Expr::Block(_) => false,
                _ => true,
            };

            arms.push(arm);

            if !input.is_empty() {
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                } else if needs_comma {
                    return Err(input.error("expected `,` after match arm"));
                }
            }
        }

        Ok(MacroInput { expr, arms })
    }
}

impl Parse for MatchArm {
    fn parse(input: ParseStream) -> Result<Self> {
        let pattern: Pat = Pat::parse_multi_with_leading_vert(input)?;

        let guard = if input.peek(Token![if]) {
            input.parse::<Token![if]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        input.parse::<Token![=>]>()?;
        let body: Expr = input.parse()?;

        Ok(MatchArm { pattern, guard, body })
    }
}
