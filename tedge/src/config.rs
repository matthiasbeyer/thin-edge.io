use nu_ansi_term::Color;
use pretty::Arena;
use pretty::Doc;
use pretty::DocAllocator;
use pretty::Pretty;
use pretty::RefDoc;
use termimad::MadSkin;

use tedge_api::config::ConfigDescription;
use tedge_api::config::ConfigEnumKind;
use tedge_api::config::ConfigKind;
use tedge_api::config::EnumVariantRepresentation;

/// Get a [`RefDoc`](pretty::RefDoc) which can be used to write the documentation of this
pub fn as_terminal_doc<'a>(desc: &'a ConfigDescription, arena: &'a Arena<'a>) -> RefDoc<'a> {
    let mut doc = arena.nil();

    if !matches!(desc.kind(), ConfigKind::Wrapped(_)) && desc.doc().is_none() {
        doc = doc
            .append(Color::LightBlue.bold().paint(desc.name()).to_string())
            .append(arena.space())
            .append(match desc.kind() {
                ConfigKind::Bool
                | ConfigKind::Integer
                | ConfigKind::Float
                | ConfigKind::String
                | ConfigKind::Wrapped(_)
                | ConfigKind::Array(_)
                | ConfigKind::HashMap(_) => arena.nil(),
                ConfigKind::Struct(_) => {
                    arena.text(Color::Blue.dimmed().paint("[Table]").to_string())
                }
                ConfigKind::Enum(_, _) => {
                    arena.text(Color::Green.dimmed().paint("[Enum]").to_string())
                }
            })
            .append(arena.hardline());
    }

    let skin = MadSkin::default_dark();
    let render_markdown = |text: &str| {
        let rendered = skin.text(text, None).to_string();
        arena.intersperse(
            rendered.split("\n").map(|t| {
                arena.intersperse(
                    t.split(char::is_whitespace).map(|t| t.to_string()),
                    arena.softline(),
                )
            }),
            arena.hardline(),
        )
    };

    if let Some(conf_doc) = desc.doc() {
        doc = doc.append(render_markdown(conf_doc));
    }

    match desc.kind() {
        ConfigKind::Bool | ConfigKind::Integer | ConfigKind::Float | ConfigKind::String => (),
        ConfigKind::Struct(stc) => {
            doc = doc
                .append(arena.hardline())
                .append(Color::Blue.paint("[Members]").to_string())
                .append(arena.hardline())
                .append(arena.intersperse(
                    stc.iter().map(|(member_name, member_doc, member_conf)| {
                        let mut doc = arena.nil();

                        if let Some(member_doc) = member_doc {
                            doc = doc.append(render_markdown(member_doc));
                        }
                        doc.append(arena.text(Color::Blue.bold().paint(*member_name).to_string()))
                            .append(": ")
                            .append(
                                Pretty::pretty(as_terminal_doc(member_conf, arena), arena).nest(4),
                            )
                    }),
                    Doc::hardline(),
                ))
        }
        ConfigKind::Enum(enum_kind, variants) => {
            doc = doc
                .append(arena.hardline())
                .append(Color::Green.paint("One of:").to_string())
                .append(arena.space())
                .append(match enum_kind {
                    ConfigEnumKind::Tagged(tag) => arena.text(
                        Color::White
                            .dimmed()
                            .paint(format!(
                                "[Tagged with {}]",
                                Color::LightGreen
                                    .italic()
                                    .dimmed()
                                    .paint(format!("'{}'", tag))
                            ))
                            .to_string(),
                    ),
                    ConfigEnumKind::Untagged => {
                        arena.text(Color::White.dimmed().paint("[Untagged]").to_string())
                    }
                })
                .append(arena.hardline())
                .append(
                    arena.intersperse(
                        variants
                            .iter()
                            .map(|(member_name, member_doc, member_conf)| {
                                arena.text("-").append(arena.space()).append({
                                    let mut doc = arena
                                        .nil()
                                        .append(match member_conf {
                                            EnumVariantRepresentation::String(rep) => arena.text(
                                                Color::Green
                                                    .bold()
                                                    .paint(&format!("{:?}", rep.to_lowercase()))
                                                    .to_string(),
                                            ),
                                            EnumVariantRepresentation::Wrapped(_) => arena.text(
                                                Color::Green.bold().paint(*member_name).to_string(),
                                            ),
                                        })
                                        .append(": ");

                                    if let Some(member_doc) = member_doc {
                                        doc = doc.append(render_markdown(member_doc));
                                    }

                                    doc.append(
                                        Pretty::pretty(
                                            match member_conf {
                                                EnumVariantRepresentation::String(_) => {
                                                    arena.nil().into_doc()
                                                }

                                                EnumVariantRepresentation::Wrapped(member_conf) => {
                                                    arena
                                                        .text(
                                                            Color::LightRed
                                                                .paint("Is a: ")
                                                                .to_string(),
                                                        )
                                                        .append(as_terminal_doc(member_conf, arena))
                                                        .into_doc()
                                                }
                                            },
                                            arena,
                                        )
                                        .nest(4),
                                    )
                                    .nest(2)
                                })
                            }),
                        Doc::hardline(),
                    ),
                );
        }
        ConfigKind::Array(conf) => {
            doc = doc
                .append(Color::LightRed.paint("Many of:").to_string())
                .append(arena.space())
                .append(as_terminal_doc(conf, arena));
        }
        ConfigKind::HashMap(conf) | ConfigKind::Wrapped(conf) => {
            doc = doc.append(as_terminal_doc(conf, arena));
        }
    };

    doc.into_doc()
}
