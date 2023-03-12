use tcod::colors::*;
use tcod::console::*;
use tcod::input::{Key, Mouse};
use tcod::map::Map as FovMap;

use crate::actions::*;

pub type Map = Vec<Vec<Tile>>;

pub struct Tcod {
    pub root: Root,
    pub con: Offscreen,
    pub panel: Offscreen,
    pub fov: FovMap,
    pub key: Key,  
    pub mouse: Mouse,
}

pub struct Game {
    pub map: Map,
    pub messages: Messages,
    pub inventory: Vec<Object>,
}

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub blocked: bool,
    pub explored: bool,
    pub block_sight: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            explored: false,
            block_sight: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            explored: false,
            block_sight: true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
        }
    }

    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }
    
    pub fn intersects_with(&self, other: &Rect) -> bool {
        // returns true if this rectangle intersects with another one
        (self.x1 <= other.x2)
            && (self.x2 >= other.x1)
            && (self.y1 <= other.y2)
            && (self.y2 >= other.y1)
    }
}

#[derive(Debug)]
pub struct Object {
    pub x: i32,
    pub y: i32,
    pub char: char,
    pub color: Color,
    pub name: String,  
    pub blocks: bool,  
    pub alive: bool,
    pub fighter: Option<Fighter>,  
    pub ai: Option<Ai>,
    pub item: Option<Item>,
}

impl Object {
    pub fn new(x: i32, y: i32, char: char, name: &str, color: Color, blocks: bool) -> Self {
        Object { 
            x, 
            y, 
            char, 
            color,
            name: name.into(),
            blocks,
            alive : false,
            fighter: None,
            ai: None,
            item: None
        }
    }

    /// set the color and then draw the character that represents this object at its position
    pub fn draw(&self, con: &mut dyn Console) {
        con.set_default_foreground(self.color);
        con.put_char(self.x, self.y, self.char, BackgroundFlag::None);
    }

    /// Get current position
    pub fn pos(&self) -> (i32, i32) {
        (self.x, self.y)
    }
    
    /// Set current position
    pub fn set_pos(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    /// return the distance to another object
    pub fn distance_to(&self, other: &Object) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        ((dx.pow(2) + dy.pow(2)) as f32).sqrt()
    }

    pub fn take_damage(&mut self, damage: i32, game: &mut Game) {
        // apply damage if possible
        if let Some(fighter) = self.fighter.as_mut() {
            if damage > 0 {
                fighter.hp -= damage;
            }
        }

        // check for death, call the death function
        if let Some(fighter) = self.fighter {
            if fighter.hp <= 0 {
                self.alive = false;
                fighter.on_death.callback(self, game);
            }
        }
    }

    pub fn attack(&mut self, target: &mut Object, game: &mut Game) {
        // a simple formula for attack damage
        let damage = self.fighter.map_or(0, |f| f.power) - target.fighter.map_or(0, |f| f.defense);
        if damage > 0 {
            // make the target take some damage
            game.messages.add(
                format!(
                    "{} attacks {} for {} hit points.",
                    self.name, target.name, damage
                ),
                WHITE,
            );
            target.take_damage(damage, game);
        } else {
            game.messages.add(
                format!(
                    "{} attacks {} but it has no effect!",
                    self.name, target.name
                ),
                WHITE,
            );
        }
    }

    /// heal by the given amount, without going over the maximum
    pub fn heal(&mut self, amount: i32) {
        if let Some(ref mut fighter) = self.fighter {
            fighter.hp += amount;
            if fighter.hp > fighter.max_hp {
                fighter.hp = fighter.max_hp;
            }
        }
    }

    /// return the distance to some coordinates
    pub fn distance(&self, x: i32, y: i32) -> f32 {
        (((x - self.x).pow(2) + (y - self.y).pow(2)) as f32).sqrt()
    }
}

// combat-related properties and methods (monster, player, NPC).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fighter {
    pub max_hp: i32,
    pub hp: i32,
    pub defense: i32,
    pub power: i32,
    pub on_death: DeathCallback,
}

pub struct Messages {
    pub messages: Vec<(String, Color)>,
}

impl Messages {
    pub fn new() -> Self {
        Self { messages: vec![] }
    }

    /// add the new message as a tuple, with the text and the color
    pub fn add<T: Into<String>>(&mut self, message: T, color: Color) {
        self.messages.push((message.into(), color));
    }

    /// Create a `DoubleEndedIterator` over the messages
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &(String, Color)> {
        self.messages.iter()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ai {
    Basic,
    Confused {
        previous_ai: Box<Ai>,
        num_turns: i32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    Heal,
    Lightning,
    Confuse,
    Fireball,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeathCallback {
    Player,
    Monster,
}

impl DeathCallback {
    pub fn callback(self, object: &mut Object, game: &mut Game) {
        use DeathCallback::*;
        let callback = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(object, game);
    }
}

pub enum UseResult {
    UsedUp,
    Cancelled,
}