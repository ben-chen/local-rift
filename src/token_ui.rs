//! Token presentation uses token identity, never the creating card's costs or rules.
use super::*;

const ART: &[(&str, &[u8])] = &[
    ("Bird", include_bytes!("../assets/art/token_bird.png")),
    ("Gold", include_bytes!("../assets/art/token_gold.png")),
    ("Mech", include_bytes!("../assets/art/token_mech.png")),
    ("Recruit", include_bytes!("../assets/art/token_recruit.png")),
    (
        "Sand Soldier",
        include_bytes!("../assets/art/token_sand_soldier.png"),
    ),
    ("Sprite", include_bytes!("../assets/art/token_sprite.png")),
    (
        "Tentacle",
        include_bytes!("../assets/art/token_tentacle.png"),
    ),
];

pub(super) struct TokenArt(std::collections::BTreeMap<&'static str, Texture2D>);
impl TokenArt {
    pub(super) fn new() -> Self {
        Self(
            ART.iter()
                .map(|&(name, bytes)| {
                    let texture = Texture2D::from_file_with_format(bytes, None);
                    texture.set_filter(FilterMode::Linear);
                    (name, texture)
                })
                .collect(),
        )
    }
    pub(super) fn get(&self, name: &str) -> Option<&Texture2D> {
        self.0.get(name.strip_suffix(" token").unwrap_or(name))
    }
}

#[derive(Clone)]
pub(super) struct PinnedToken {
    pub owner: usize,
    pub object_id: u32,
    pub source_card: usize,
    pub preview: TokenPreview,
}

fn preview(game: &Game, owner: usize, object_id: u32, source_card: usize) -> Option<TokenPreview> {
    let player = game.players.get(owner)?;
    if let Some(unit) = player
        .units
        .iter()
        .find(|u| u.id == object_id && u.card_id == source_card && u.token)
    {
        return Some(TokenPreview {
            name: game.unit_name(unit),
            might: game.unit_might(unit).ok()?,
            gear: false,
            rules: token_rules(unit.card_id),
        });
    }
    player
        .gear
        .iter()
        .find(|g| g.id == object_id && g.card_id == source_card && g.is_gold())
        .map(|_| TokenPreview {
            name: "Gold".into(),
            might: 0,
            gear: true,
            rules: GOLD_RULES.into(),
        })
}

impl App {
    /// Crop inside the original artwork, excluding printed language and statistics.
    pub(super) fn token_picture(&self, rect: Rect, name: &str) {
        if let Some(texture) = self.token_art.get(name) {
            let bounds = Rect::new(
                texture.width() * 0.08,
                texture.height() * 0.14,
                texture.width() * 0.84,
                texture.height() * 0.48,
            );
            let scale = (bounds.w / rect.w).min(bounds.h / rect.h);
            let crop = Rect::new(
                bounds.x + (bounds.w - rect.w * scale) / 2.0,
                bounds.y + (bounds.h - rect.h * scale) / 2.0,
                rect.w * scale,
                rect.h * scale,
            );
            self.ui.image(texture, rect, Some(crop), WHITE);
        } else {
            self.ui.rect(rect, BG);
            self.ui.diamond(
                vec2(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0),
                20.0,
                TEAL,
            );
        }
    }

    pub(super) fn pin_token(&mut self, owner: usize, object_id: u32, source_card: usize) {
        if let Some(preview) = preview(&self.game, owner, object_id, source_card) {
            self.pinned_token = Some(PinnedToken {
                owner,
                object_id,
                source_card,
                preview,
            });
            self.hover_object = Some((owner, object_id));
            self.inspect = Some(source_card);
            self.inspect_scroll = 0;
        }
    }

    pub(super) fn token_inspector(&mut self, pinned: PinnedToken) {
        self.ui.rect(
            Rect::new(0.0, 0.0, 1440.0, 900.0),
            Color::new(0.0, 0.015, 0.02, 0.88),
        );
        self.ui.enabled = true;
        let r = Rect::new(348.0, 139.0, 842.0, 622.0);
        self.ui.rect(r, PANEL);
        self.ui.border(r, LINE);
        self.token_picture(Rect::new(372.0, 164.0, 360.0, 360.0), &pinned.preview.name);
        let current = preview(
            &self.game,
            pinned.owner,
            pinned.object_id,
            pinned.source_card,
        );
        let token = current.as_ref().unwrap_or(&pinned.preview);
        self.ui.pill(764.0, 174.0, "TOKEN INSPECTOR", TEAL);
        self.ui
            .wrapped(&token.name, 764.0, 239.0, 390.0, 25.0, TEXT);
        self.ui.text(
            if token.gear {
                "Domainless · Gear token"
            } else {
                "Domainless · Unit token"
            },
            764.0,
            296.0,
            13.0,
            GOLD,
        );
        self.ui
            .text("No printed Energy or Power cost", 764.0, 324.0, 11.0, MUTED);
        self.ui
            .line(vec2(764.0, 348.0), vec2(1158.0, 348.0), 1.0, LINE);
        self.ui
            .bold("LIVE STATE & TOKEN RULES", 764.0, 378.0, 11.0, TEAL);
        let text = if current.is_some() {
            live_object_description(
                &self.game,
                pinned.owner,
                pinned.object_id,
                pinned.source_card,
            )
            .unwrap_or_else(|| token.rules.clone())
        } else {
            format!("This token is no longer on the board.\n\n{}", token.rules)
        };
        self.ui.scroll_text(
            &text,
            Rect::new(764.0, 400.0, 394.0, 250.0),
            13.0,
            &mut self.inspect_scroll,
        );
        self.ui.wrapped(
            "Tokens leave play permanently when they move to a hand, deck, trash, or banishment.",
            372.0,
            568.0,
            360.0,
            13.0,
            MUTED,
        );
        self.ui
            .text("Official token art © Riot Games", 372.0, 705.0, 10.0, MUTED);
        if self.ui.button(
            Rect::new(764.0, 682.0, 394.0, 44.0),
            "Back to the game  [ESC]",
            true,
            true,
        ) {
            self.inspect = None;
            self.pinned_token = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use local_rift::game::Unit;

    #[test]
    fn every_supported_token_identity_has_its_own_artwork() {
        for id in 0..local_rift::cards::cards().len() {
            if let Some(spec) = local_rift::rules::token_spec(id) {
                assert!(
                    ART.iter().any(|&(name, bytes)| name == spec.name
                        && bytes.starts_with(b"\x89PNG\r\n\x1a\n")),
                    "{id} / {}",
                    spec.name
                );
            }
        }
    }

    #[test]
    fn live_token_preview_uses_physical_identity_and_never_inherits_creator_rules() {
        let mut game = Game::new_for_agents(3, 3, 448, Difficulty::Easy);
        for (slot, source) in [448, 355, 34, 48, 54, 87].into_iter().enumerate() {
            let spec = local_rift::rules::token_spec(source).unwrap();
            let id = 9000 + slot as u32;
            game.players[0].units.push(Unit {
                id,
                card_id: source,
                lane: None,
                ready: false,
                damage: 0,
                buff: spec.might.into(),
                temporary_buff: 2,
                token: true,
                protected: false,
                stunned: false,
                banish_on_death: false,
            });
            let token = preview(&game, 0, id, source).unwrap();
            assert!(token.name.starts_with(spec.name));
            assert!(token.rules.contains("domainless"));
            assert_eq!(token.might, u64::from(spec.might) + 2);
            assert!(preview(&game, 1, id, source).is_none());
            assert!(preview(&game, 0, id, source + 1).is_none());
            game.players[0].units.last_mut().unwrap().token = false;
            assert!(preview(&game, 0, id, source).is_none());
        }
    }
}
