#![no_std]

use asr::{
    gba,
    time_util::frame_count,
    timer::{self, TimerState},
    watcher::Pair,
};
use bytemuck::{Pod, Zeroable};
use spinning_top::{const_spinlock, Spinlock};

#[cfg(all(not(test), target_arch = "wasm32"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

static STATE: Spinlock<State> = const_spinlock(State { game: None });

struct State {
    game: Option<Game>,
}

struct Game {
    emulator: gba::Emulator,
    pause_menu: Watcher<PauseMenu>,
    scene: Watcher<Scene>,
    dhc_big_key: Watcher<i32>,
    vaati3_phases: Watcher<i32>,
    sprite: Watcher<Sprite>,
    frame_count: Watcher<u16>,
    uix_position: Watcher<i32>,
    uiy_position: Watcher<i32>,
    visual_rupees: Watcher<u16>,
    visual_hearts: Watcher<u8>,
    visual_keys: Watcher<u8>,
    tiger_scrolls: Watcher<u8>,
    mysterious_shells: Watcher<u16>,
    bombs: Watcher<u8>,
    accumulated_frame_count: i64,
    delayed_split: Option<(&'static str, i64)>,
    visited_scenes: VisitedScenes,
}

#[derive(Default)]
struct VisitedScenes {
    deepwood_shrine: bool,
    deepwood_shrine_boss: bool,
    mt_crenel: bool,
    cave_of_flames: bool,
    cave_of_flames_boss: bool,
    fortress_of_winds: bool,
    temple_of_droplets: bool,
    palace_of_winds: bool,
}

impl Game {
    fn new_ntscj(emulator: gba::Emulator) -> Self {
        Self {
            emulator,
            pause_menu: Watcher::new(0x2002B32),
            scene: Watcher::new(0x3000BF4),
            dhc_big_key: Watcher::new(0x2002EB2),
            vaati3_phases: Watcher::new(0x30017BC),
            sprite: Watcher::new(0x300116C),
            frame_count: Watcher::new(0x300100C),
            uix_position: Watcher::new(0x3001E4E),
            uiy_position: Watcher::new(0x300187A),
            visual_rupees: Watcher::new(0x200AF0E),
            visual_hearts: Watcher::new(0x200AF03),
            visual_keys: Watcher::new(0x200AF12),
            tiger_scrolls: Watcher::new(0x2002B44),
            mysterious_shells: Watcher::new(0x2002B02),
            bombs: Watcher::new(0x2002AEC),
            accumulated_frame_count: 0,
            delayed_split: None,
            visited_scenes: Default::default(),
        }
    }

    fn update_vars(&mut self) -> Option<Vars<'_>> {
        Some(Vars {
            pause_menu: self.pause_menu.update(&self.emulator)?,
            scene: self.scene.update(&self.emulator)?,
            dhc_big_key: self.dhc_big_key.update(&self.emulator)?,
            vaati3_phases: self.vaati3_phases.update(&self.emulator)?,
            sprite: self.sprite.update(&self.emulator)?,
            frame_count: self.frame_count.update(&self.emulator)?,
            uix_position: self.uix_position.update(&self.emulator)?,
            uiy_position: self.uiy_position.update(&self.emulator)?,
            visual_rupees: self.visual_rupees.update(&self.emulator)?,
            visual_hearts: self.visual_hearts.update(&self.emulator)?,
            visual_keys: self.visual_keys.update(&self.emulator)?,
            tiger_scrolls: self.tiger_scrolls.update(&self.emulator)?,
            mysterious_shells: self.mysterious_shells.update(&self.emulator)?,
            bombs: self.bombs.update(&self.emulator)?,
            accumulated_frame_count: &mut self.accumulated_frame_count,
            delayed_split: &mut self.delayed_split,
            visited_scenes: &mut self.visited_scenes,
        })
    }
}

struct Vars<'a> {
    pause_menu: &'a Pair<PauseMenu>,
    scene: &'a Pair<Scene>,
    dhc_big_key: &'a Pair<i32>,
    vaati3_phases: &'a Pair<i32>,
    sprite: &'a Pair<Sprite>,
    frame_count: &'a Pair<u16>,
    uix_position: &'a Pair<i32>,
    uiy_position: &'a Pair<i32>,
    visual_rupees: &'a Pair<u16>,
    visual_hearts: &'a Pair<u8>,
    visual_keys: &'a Pair<u8>,
    tiger_scrolls: &'a Pair<u8>,
    mysterious_shells: &'a Pair<u16>,
    bombs: &'a Pair<u8>,
    accumulated_frame_count: &'a mut i64,
    delayed_split: &'a mut Option<(&'static str, i64)>,
    visited_scenes: &'a mut VisitedScenes,
}

impl Vars<'_> {
    fn frame_count(&self) -> i64 {
        *self.accumulated_frame_count + self.frame_count.current as i64
    }
}

struct Watcher<T> {
    watcher: asr::watcher::Watcher<T>,
    address: u32,
}

impl<T: Pod> Watcher<T> {
    fn new(address: u32) -> Self {
        Self {
            watcher: asr::watcher::Watcher::new(),
            address,
        }
    }

    fn update(&mut self, emulator: &gba::Emulator) -> Option<&Pair<T>> {
        self.watcher.update(emulator.read(self.address).ok())
    }
}

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct PauseMenu {
    inventory: [InventoryItem; 6],
    _unknown: [u8; 10],
    elements: Elements,
    permanent_equipment: PermanentEquipment,
}

impl PauseMenu {
    fn has_item(&self, inventory_slot: usize, inventory_item: InventoryItem) -> bool {
        self.inventory[inventory_slot].contains(inventory_item)
    }
}

bitflags::bitflags! {
    #[derive(Pod, Zeroable)]
    #[repr(C)]
    struct InventoryItem: u8 {
        const PAUSE_MENU = 1 << 0;
        const SMITHS_SWORD = 1 << 2;
        const WHITE_SWORD = 1 << 4;
        const WHITE_SWORD2 = 1 << 6;

        const WHITE_SWORD3 = 1 << 0;
        const SWORD_LAMP = 1 << 2;
        const FOUR_SWORD = 1 << 4;
        const BOMBS = 1 << 6;

        const REMOTE_BOMBS = 1 << 0;
        const BOW = 1 << 2;
        const BOW2 = 1 << 4;
        const BOOMERANG = 1 << 6;

        const BOOMERANG2 = 1 << 0;
        const SHIELD = 1 << 2;
        const MIRROR_SHIELD = 1 << 4;
        const LAMP = 1 << 6;

        const LAMP2 = 1 << 0;
        const GUST_JAR = 1 << 2;
        const CANE_OF_PACCI = 1 << 4;
        const MOLE_MITTS = 1 << 6;

        const ROCS_CAPE = 1 << 0;
        const PEGASUS_BOOTS = 1 << 2;
        const UNKNOWN = 1 << 4;
        const OCARINA = 1 << 6;
    }

    #[derive(Pod, Zeroable)]
    #[repr(C)]
    struct Elements: u8 {
        const EARTH = 1 << 0;
        const FIRE = 1 << 2;
        const WATER = 1 << 4;
        const WIND = 1 << 6;
    }

    #[derive(Pod, Zeroable)]
    #[repr(C)]
    struct PermanentEquipment: u8 {
        const GRIP_RING = 1 << 0;
        const POWER_BRACELETS = 1 << 2;
        const FLIPPERS = 1 << 4;
        const UNKNOWN = 1 << 6;
    }
}

#[derive(Copy, Clone, Pod, Zeroable, PartialEq, Eq)]
#[repr(transparent)]
struct Scene(u8);

#[allow(unused)]
impl Scene {
    const TITLE_SCREEN: Self = Self(0);
    const MINISH_WOODS: Self = Self(0);
    const MINISH_VILLAGE: Self = Self(0x01);
    const MARKET_PLACE: Self = Self(0x02);
    const OVERWORLD: Self = Self(0x03);
    const MT_CRENEL: Self = Self(0x06);
    const COURTYARD: Self = Self(0x07);
    const MELARIS_MINES: Self = Self(0x10);
    const MARKET_PLACE_INTRO: Self = Self(0x15);
    const FORTRESS_OF_WINDS: Self = Self(0x18);
    const HOUSE: Self = Self(0x20);
    const LINKS_HOUSE: Self = Self(0x22);
    const DEEPWOOD_SHRINE: Self = Self(0x48);
    const DEEPWOOD_SHRINE_BOSS: Self = Self(0x49);
    const CAVE_OF_FLAMES: Self = Self(0x50);
    const CAVE_OF_FLAMES_BOSS: Self = Self(0x51);
    const TEMPLE_OF_DROPLETS: Self = Self(0x60);
    const PALACE_OF_WINDS: Self = Self(0x70);
    const HYRULE_CASTLE: Self = Self(0x80);
    const DARK_HYRULE_CASTLE: Self = Self(0x88);
    const VAATI3: Self = Self(0x8B);
}

#[derive(Copy, Clone, Pod, Zeroable, PartialEq, Eq)]
#[repr(transparent)]
struct Sprite(u16);

impl Sprite {
    const RECEIVE_MINISH_CAP: Self = Self(0x31C);
}

#[allow(unused)]
mod inventory_slot {
    pub const PAUSE_MENU: usize = 0;
    pub const SMITHS_SWORD: usize = 0;
    pub const WHITE_SWORD: usize = 0;
    pub const WHITE_SWORD2: usize = 0;

    pub const WHITE_SWORD3: usize = 1;
    pub const SWORD_LAMP: usize = 1;
    pub const FOUR_SWORD: usize = 1;
    pub const BOMBS: usize = 1;

    pub const REMOTE_BOMBS: usize = 2;
    pub const BOW: usize = 2;
    pub const BOW2: usize = 2;
    pub const BOOMERANG: usize = 2;

    pub const BOOMERANG2: usize = 3;
    pub const SHIELD: usize = 3;
    pub const MIRROR_SHIELD: usize = 3;
    pub const LAMP: usize = 3;

    pub const LAMP2: usize = 4;
    pub const GUST_JAR: usize = 4;
    pub const CANE_OF_PACCI: usize = 4;
    pub const MOLE_MITTS: usize = 4;

    pub const ROCS_CAPE: usize = 5;
    pub const PEGASUS_BOOTS: usize = 5;
    pub const UNKNOWN: usize = 5;
    pub const OCARINA: usize = 5;
}

#[no_mangle]
pub extern "C" fn update() {
    let mut state = STATE.lock();
    if state.game.is_none() {
        state.game = gba::Emulator::attach().map(Game::new_ntscj);
    }
    if let Some(game) = &mut state.game {
        if !game.emulator.is_open() {
            state.game = None;
            return;
        }
        if let Some(mut vars) = game.update_vars() {
            timer::set_variable_int("Hearts", vars.visual_hearts.current);
            timer::set_variable_int("Rupees", vars.visual_rupees.current);
            timer::set_variable_int("Keys", vars.visual_keys.current);
            timer::set_variable_int("Tiger Scrolls", vars.tiger_scrolls.current);
            timer::set_variable_int("Mysterious Shells", vars.mysterious_shells.current);
            timer::set_variable_int("Bombs", vars.bombs.current);

            match timer::state() {
                TimerState::NotRunning => {
                    if vars.uix_position.current == 24
                        && vars.uiy_position.old == 144
                        && vars.uiy_position.current > 144
                    {
                        *vars.accumulated_frame_count = -(vars.frame_count.current as i64);
                        *vars.visited_scenes = Default::default();
                        timer::start();
                        timer::pause_game_time();
                    }
                }
                TimerState::Running | TimerState::Paused => {
                    if vars.frame_count.current < vars.frame_count.old {
                        *vars.accumulated_frame_count += vars.frame_count.old as i64 + 1;
                    }

                    timer::set_game_time(frame_count::<60>(vars.frame_count() as u64));

                    if let Some(reason) = should_split(&mut vars) {
                        asr::print_message(reason);
                        timer::split();
                    }
                }
                _ => {}
            }
        }
    }
}

fn should_split(vars: &mut Vars) -> Option<&'static str> {
    if let Some((message, time_stamp)) = *vars.delayed_split {
        if vars.frame_count() >= time_stamp {
            *vars.delayed_split = None;
            return Some(message);
        }
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::SMITHS_SWORD, InventoryItem::SMITHS_SWORD))
    {
        // Get Smith's Sword
        return Some("Get Smith's Sword");
    }
    if vars
        .sprite
        .check(|&sprite| sprite == Sprite::RECEIVE_MINISH_CAP)
        && vars.scene.current == Scene::MINISH_WOODS
    {
        // Receive Minish Cap
        *vars.delayed_split = Some(("Receive Minish Cap", vars.frame_count() + 20));
        return None;
    }
    if !vars.visited_scenes.deepwood_shrine && vars.scene.current == Scene::DEEPWOOD_SHRINE {
        // Enter Deepwood Shrine
        vars.visited_scenes.deepwood_shrine = true;
        return Some("Enter Deepwood Shrine");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::GUST_JAR, InventoryItem::GUST_JAR))
    {
        // Get Gust Jar
        return Some("Get Gust Jar");
    }
    if !vars.visited_scenes.deepwood_shrine_boss
        && vars.scene.current == Scene::DEEPWOOD_SHRINE_BOSS
    {
        // Enter Deepwood Shrine Boss Room
        vars.visited_scenes.deepwood_shrine_boss = true;
        return Some("Enter Deepwood Shrine Boss Room");
    }
    if vars
        .pause_menu
        .check(|menu| menu.elements.contains(Elements::EARTH))
    {
        // Get Earth Element
        return Some("Get Earth Element");
    }
    if !vars.visited_scenes.mt_crenel && vars.scene.current == Scene::MT_CRENEL {
        // Enter Mt. Crenel
        vars.visited_scenes.mt_crenel = true;
        return Some("Enter Mt. Crenel");
    }
    if vars.pause_menu.check(|menu| {
        menu.permanent_equipment
            .contains(PermanentEquipment::GRIP_RING)
    }) {
        // Get Grip Ring
        return Some("Get Grip Ring");
    }
    if !vars.visited_scenes.cave_of_flames && vars.scene.current == Scene::CAVE_OF_FLAMES {
        // Enter Cave of Flames
        vars.visited_scenes.cave_of_flames = true;
        return Some("Enter Cave of Flames");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::CANE_OF_PACCI, InventoryItem::CANE_OF_PACCI))
    {
        // Get Cane of Pacci
        return Some("Get Cane of Pacci");
    }
    if !vars.visited_scenes.cave_of_flames_boss && vars.scene.current == Scene::CAVE_OF_FLAMES_BOSS
    {
        // Enter Cave of Flames Boss Room
        vars.visited_scenes.cave_of_flames_boss = true;
        return Some("Enter Cave of Flames Boss Room");
    }
    if vars
        .pause_menu
        .check(|menu| menu.elements.contains(Elements::FIRE))
    {
        // Get Fire Element
        return Some("Get Fire Element");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::PEGASUS_BOOTS, InventoryItem::PEGASUS_BOOTS))
    {
        // Get Pegasus Boots
        return Some("Get Pegasus Boots");
    }
    if !vars.visited_scenes.fortress_of_winds && vars.scene.current == Scene::FORTRESS_OF_WINDS {
        // Enter Fortress of Winds
        vars.visited_scenes.fortress_of_winds = true;
        return Some("Enter Fortress of Winds");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::MOLE_MITTS, InventoryItem::MOLE_MITTS))
    {
        // Get Mole Mitts
        return Some("Get Mole Mitts");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::OCARINA, InventoryItem::OCARINA))
    {
        // Get Ocarina
        return Some("Get Ocarina");
    }
    if vars.pause_menu.check(|menu| {
        menu.permanent_equipment
            .contains(PermanentEquipment::FLIPPERS)
    }) {
        // Get Flippers
        return Some("Get Flippers");
    }
    if !vars.visited_scenes.temple_of_droplets && vars.scene.current == Scene::TEMPLE_OF_DROPLETS {
        // Enter Temple of Droplets
        vars.visited_scenes.temple_of_droplets = true;
        return Some("Enter Temple of Droplets");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::LAMP, InventoryItem::LAMP))
    {
        // Get Lamp
        return Some("Get Lamp");
    }
    if vars
        .pause_menu
        .check(|menu| menu.elements.contains(Elements::WATER))
    {
        // Get Water Element
        return Some("Get Water Element");
    }
    if !vars.visited_scenes.palace_of_winds && vars.scene.current == Scene::PALACE_OF_WINDS {
        // Enter Palace of Winds
        vars.visited_scenes.palace_of_winds = true;
        return Some("Enter Palace of Winds");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::ROCS_CAPE, InventoryItem::ROCS_CAPE))
    {
        // Get Roc's Cape
        return Some("Get Roc's Cape");
    }
    if vars
        .pause_menu
        .check(|menu| menu.elements.contains(Elements::WIND))
    {
        // Get Wind Element
        return Some("Get Wind Element");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::FOUR_SWORD, InventoryItem::FOUR_SWORD))
    {
        // Get Four Sword
        *vars.delayed_split = Some(("Get Four Sword", vars.frame_count() + 244));
        return None;
    }
    if vars.dhc_big_key.check(|&v| v & 4 != 0) {
        // Get DHC Big Key
        return Some("Get DHC Big Key");
    }
    if vars.scene.current == Scene::VAATI3
        && vars.vaati3_phases.old == 1
        && vars.vaati3_phases.current == 0
    {
        // Defeat Vaati
        return Some("Defeat Vaati");
    }
    None
}
