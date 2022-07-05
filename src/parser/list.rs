use crate::{
    core::receiver::{Request, Token},
    protocol::{
        list::{self, ReturnOption, SelectionOption},
        status::Status,
    },
};

#[allow(clippy::while_let_on_iterator)]
pub fn parse_list(request: Request) -> crate::core::Result<list::Arguments> {
    match request.tokens.len() {
        0 | 1 => Err(request.into_error("Missing arguments.")),
        2 => {
            let mut tokens = request.tokens.into_iter();
            Ok(list::Arguments::Basic {
                reference_name: tokens
                    .next()
                    .unwrap()
                    .unwrap_string()
                    .map_err(|v| (request.tag.as_str(), v))?,
                mailbox_name: tokens
                    .next()
                    .unwrap()
                    .unwrap_string()
                    .map_err(|v| (request.tag.as_str(), v))?,
                tag: request.tag,
            })
        }
        _ => {
            let mut tokens = request.tokens.into_iter();
            let mut selection_options = Vec::new();
            let mut return_options = Vec::new();
            let mut mailbox_name = Vec::new();

            let reference_name = match tokens.next().unwrap() {
                Token::ParenthesisOpen => {
                    while let Some(token) = tokens.next() {
                        match token {
                            Token::ParenthesisClose => break,
                            Token::Argument(value) => {
                                selection_options.push(
                                    SelectionOption::parse(&value)
                                        .map_err(|v| (request.tag.as_str(), v))?,
                                );
                            }
                            _ => {
                                return Err((
                                    request.tag.as_str(),
                                    "Invalid selection option argument.",
                                )
                                    .into())
                            }
                        }
                    }
                    tokens
                        .next()
                        .ok_or((request.tag.as_str(), "Missing reference name."))?
                        .unwrap_string()
                        .map_err(|v| (request.tag.as_str(), v))?
                }
                token => token
                    .unwrap_string()
                    .map_err(|v| (request.tag.as_str(), v))?,
            };

            match tokens
                .next()
                .ok_or((request.tag.as_str(), "Missing mailbox name."))?
            {
                Token::ParenthesisOpen => {
                    while let Some(token) = tokens.next() {
                        match token {
                            Token::ParenthesisClose => break,
                            token => {
                                mailbox_name.push(
                                    token
                                        .unwrap_string()
                                        .map_err(|v| (request.tag.as_str(), v))?,
                                );
                            }
                        }
                    }
                }
                token => {
                    mailbox_name.push(
                        token
                            .unwrap_string()
                            .map_err(|v| (request.tag.as_str(), v))?,
                    );
                }
            }

            if tokens
                .next()
                .map_or(false, |token| token.eq_ignore_ascii_case(b"return"))
            {
                if tokens
                    .next()
                    .map_or(true, |token| !token.is_parenthesis_open())
                {
                    return Err((
                        request.tag.as_str(),
                        "Invalid return option, expected parenthesis.",
                    )
                        .into());
                }

                while let Some(token) = tokens.next() {
                    match token {
                        Token::ParenthesisClose => break,
                        Token::Argument(value) => {
                            let mut return_option = ReturnOption::parse(&value)
                                .map_err(|v| (request.tag.as_str(), v))?;
                            if let ReturnOption::Status(status) = &mut return_option {
                                if tokens
                                    .next()
                                    .map_or(true, |token| !token.is_parenthesis_open())
                                {
                                    return Err((
                                        request.tag,
                                        "Invalid return option, expected parenthesis after STATUS.",
                                    )
                                        .into());
                                }
                                while let Some(token) = tokens.next() {
                                    match token {
                                        Token::ParenthesisClose => break,
                                        Token::Argument(value) => {
                                            status.push(
                                                Status::parse(&value)
                                                    .map_err(|v| (request.tag.as_str(), v))?,
                                            );
                                        }
                                        _ => {
                                            return Err((
                                                request.tag,
                                                "Invalid status return option argument.",
                                            )
                                                .into())
                                        }
                                    }
                                }
                            }
                            return_options.push(return_option);
                        }
                        _ => {
                            return Err(
                                (request.tag.as_str(), "Invalid return option argument.").into()
                            )
                        }
                    }
                }
            }

            Ok(list::Arguments::Extended {
                tag: request.tag,
                reference_name,
                mailbox_name,
                selection_options,
                return_options,
            })
        }
    }
}

impl SelectionOption {
    pub fn parse(value: &[u8]) -> super::Result<Self> {
        if value.eq_ignore_ascii_case(b"subscribed") {
            Ok(Self::Subscribed)
        } else if value.eq_ignore_ascii_case(b"remote") {
            Ok(Self::Remote)
        } else if value.eq_ignore_ascii_case(b"recursivematch") {
            Ok(Self::RecursiveMatch)
        } else {
            Err(format!(
                "Invalid selection option {:?}.",
                String::from_utf8_lossy(value)
            )
            .into())
        }
    }
}

impl ReturnOption {
    pub fn parse(value: &[u8]) -> super::Result<Self> {
        if value.eq_ignore_ascii_case(b"subscribed") {
            Ok(Self::Subscribed)
        } else if value.eq_ignore_ascii_case(b"children") {
            Ok(Self::Children)
        } else if value.eq_ignore_ascii_case(b"status") {
            Ok(Self::Status(Vec::with_capacity(2)))
        } else {
            Err(format!("Invalid return option {:?}", String::from_utf8_lossy(value)).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        core::receiver::Receiver,
        protocol::{
            list::{self, ReturnOption, SelectionOption},
            status::Status,
        },
    };

    #[test]
    fn parse_list() {
        let mut receiver = Receiver::new();

        for (command, arguments) in [
            (
                "A682 LIST \"\" *\r\n",
                list::Arguments::Basic {
                    tag: "A682".to_string(),
                    reference_name: "".to_string(),
                    mailbox_name: "*".to_string(),
                },
            ),
            (
                "A02 LIST (SUBSCRIBED) \"\" \"*\"\r\n",
                list::Arguments::Extended {
                    tag: "A02".to_string(),
                    reference_name: "".to_string(),
                    mailbox_name: vec!["*".to_string()],
                    selection_options: vec![SelectionOption::Subscribed],
                    return_options: vec![],
                },
            ),
            (
                "A03 LIST () \"\" \"%\" RETURN (CHILDREN)\r\n",
                list::Arguments::Extended {
                    tag: "A03".to_string(),
                    reference_name: "".to_string(),
                    mailbox_name: vec!["%".to_string()],
                    selection_options: vec![],
                    return_options: vec![ReturnOption::Children],
                },
            ),
            (
                "A04 LIST (REMOTE) \"\" \"%\" RETURN (CHILDREN)\r\n",
                list::Arguments::Extended {
                    tag: "A04".to_string(),
                    reference_name: "".to_string(),
                    mailbox_name: vec!["%".to_string()],
                    selection_options: vec![SelectionOption::Remote],
                    return_options: vec![ReturnOption::Children],
                },
            ),
            (
                "A05 LIST (REMOTE SUBSCRIBED) \"\" \"*\"\r\n",
                list::Arguments::Extended {
                    tag: "A05".to_string(),
                    reference_name: "".to_string(),
                    mailbox_name: vec!["*".to_string()],
                    selection_options: vec![SelectionOption::Remote, SelectionOption::Subscribed],
                    return_options: vec![],
                },
            ),
            (
                "A06 LIST (REMOTE) \"\" \"*\" RETURN (SUBSCRIBED)\r\n",
                list::Arguments::Extended {
                    tag: "A06".to_string(),
                    reference_name: "".to_string(),
                    mailbox_name: vec!["*".to_string()],
                    selection_options: vec![SelectionOption::Remote],
                    return_options: vec![ReturnOption::Subscribed],
                },
            ),
            (
                "C04 LIST (SUBSCRIBED RECURSIVEMATCH) \"\" \"%\"\r\n",
                list::Arguments::Extended {
                    tag: "C04".to_string(),
                    reference_name: "".to_string(),
                    mailbox_name: vec!["%".to_string()],
                    selection_options: vec![
                        SelectionOption::Subscribed,
                        SelectionOption::RecursiveMatch,
                    ],
                    return_options: vec![],
                },
            ),
            (
                "C04 LIST (SUBSCRIBED RECURSIVEMATCH) \"\" \"%\" RETURN (CHILDREN)\r\n",
                list::Arguments::Extended {
                    tag: "C04".to_string(),
                    reference_name: "".to_string(),
                    mailbox_name: vec!["%".to_string()],
                    selection_options: vec![
                        SelectionOption::Subscribed,
                        SelectionOption::RecursiveMatch,
                    ],
                    return_options: vec![ReturnOption::Children],
                },
            ),
            (
                "a1 LIST \"\" (\"foo\")\r\n",
                list::Arguments::Extended {
                    tag: "a1".to_string(),
                    reference_name: "".to_string(),
                    mailbox_name: vec!["foo".to_string()],
                    selection_options: vec![],
                    return_options: vec![],
                },
            ),
            (
                "a3.1 LIST \"\" (% music/rock)\r\n",
                list::Arguments::Extended {
                    tag: "a3.1".to_string(),
                    reference_name: "".to_string(),
                    mailbox_name: vec!["%".to_string(), "music/rock".to_string()],
                    selection_options: vec![],
                    return_options: vec![],
                },
            ),
            (
                "BBB LIST \"\" (\"INBOX\" \"Drafts\" \"Sent/%\")\r\n",
                list::Arguments::Extended {
                    tag: "BBB".to_string(),
                    reference_name: "".to_string(),
                    mailbox_name: vec![
                        "INBOX".to_string(),
                        "Drafts".to_string(),
                        "Sent/%".to_string(),
                    ],
                    selection_options: vec![],
                    return_options: vec![],
                },
            ),
            (
                "A01 LIST \"\" % RETURN (STATUS (MESSAGES UNSEEN))\r\n",
                list::Arguments::Extended {
                    tag: "A01".to_string(),
                    reference_name: "".to_string(),
                    mailbox_name: vec!["%".to_string()],
                    selection_options: vec![],
                    return_options: vec![ReturnOption::Status(vec![
                        Status::Messages,
                        Status::Unseen,
                    ])],
                },
            ),
            (
                concat!(
                    "A02 LIST (SUBSCRIBED RECURSIVEMATCH) \"\" ",
                    "% RETURN (CHILDREN STATUS (MESSAGES))\r\n"
                ),
                list::Arguments::Extended {
                    tag: "A02".to_string(),
                    reference_name: "".to_string(),
                    mailbox_name: vec!["%".to_string()],
                    selection_options: vec![
                        SelectionOption::Subscribed,
                        SelectionOption::RecursiveMatch,
                    ],
                    return_options: vec![
                        ReturnOption::Children,
                        ReturnOption::Status(vec![Status::Messages]),
                    ],
                },
            ),
        ] {
            assert_eq!(
                super::parse_list(receiver.parse(&mut command.as_bytes().iter()).unwrap()).unwrap(),
                arguments
            );
        }
    }
}
