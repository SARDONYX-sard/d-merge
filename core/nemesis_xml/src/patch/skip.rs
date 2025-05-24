use winnow::error::ErrMode;
use winnow::prelude::*;
use winnow::token::take_until;

/// Skip the given tag (`<hkparam>` or `<hkobject>`) to the corresponding closing tag.
#[allow(unused)]
fn skip_tag_balanced<'s>(
    tag_name: &'static str,
) -> impl Parser<&'s str, (), ErrMode<winnow::error::ContextError>> {
    move |input: &mut &'s str| {
        let open_tag = format!("<{}", tag_name);
        let close_tag = format!("</{}", tag_name);
        let mut depth = 0;

        loop {
            // Search: start tag or end tag
            let next_pos = input
                .find(&open_tag)
                .map(|start| (start, true))
                .into_iter()
                .chain(input.find(&close_tag).map(|start| (start, false)))
                .min_by_key(|(start, _)| *start);

            if let Some((start, is_open)) = next_pos {
                // consume until tag
                let (_, rest) = input.split_at(start);
                *input = rest;

                if is_open {
                    open_tag.as_str().parse_next(input)?;
                    take_until(0.., ">").parse_next(input)?;
                    ">".parse_next(input)?;
                    depth += 1;
                } else {
                    close_tag.as_str().parse_next(input)?;
                    take_until(0.., ">").parse_next(input)?;
                    ">".parse_next(input)?;
                    depth -= 1;
                    if depth == 0 {
                        break Ok(());
                    }
                }
            } else {
                let ctx_err = winnow::error::ContextError::from_input(input);
                return Err(winnow::error::ErrMode::Backtrack(ctx_err));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_single_hkparam() {
        let mut input = r#"<hkparam name="foo">value</hkparam>rest"#;
        skip_tag_balanced("hkparam").parse_next(&mut input).unwrap();
        assert_eq!(input, "rest");
    }

    #[test]
    fn test_skip_nested_hkparam() {
        let mut input = r###"
		<hkobject name="#0008" class="hkRootLevelContainer" signature="0x2772c11e">
			<hkparam name="namedVariants" numelements="1">
				<hkobject>
<!-- MOD_CODE ~id~ OPEN -->
					<hkparam name="name">ReplaceDummy</hkparam>
<!-- ORIGINAL -->
					<hkparam name="name">hkbProjectData</hkparam>
<!-- CLOSE -->
					<hkparam name="className">hkbProjectData</hkparam>
					<hkparam name="variant">#0010</hkparam>
				</hkobject>
			</hkparam>
		</hkobject>
"###;
        skip_tag_balanced("hkparam").parse_next(&mut input).unwrap();
        assert_eq!(input.trim_start(), "</hkobject>\n");
    }

    #[test]
    fn test_skip_multiple_nested_levels() {
        let mut input = r#"
        <hkparam name="a">
            <hkparam name="b">
                <hkparam name="c">deep</hkparam>
            </hkparam>
        </hkparam>
        done"#;
        skip_tag_balanced("hkparam").parse_next(&mut input).unwrap();
        assert_eq!(input.trim_start(), "done");
    }

    #[test]
    fn test_skip_with_sibling_tags() {
        let mut input = r#"
        <hkparam name="a">
            <hkparam name="b">value</hkparam>
            <hkparam name="c">value</hkparam>
        </hkparam>
        trailing"#;
        skip_tag_balanced("hkparam").parse_next(&mut input).unwrap();
        assert_eq!(input.trim_start(), "trailing");
    }
}
