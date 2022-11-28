#![no_std]

use arrayvec::ArrayString;
use asr::{
    gba, itoa,
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

static STATE: Spinlock<State> = const_spinlock(State {
    game: None,
    settings: None,
});

#[derive(asr::Settings)]
struct Settings {
    /// Get Smith's Sword
    #[default = true]
    get_smiths_sword: bool,
    /// Receive Minish Cap
    #[default = true]
    receive_minish_cap: bool,
    /// Enter Deepwood Shrine
    #[default = true]
    enter_deepwood_shrine: bool,
    /// Get Gust Jar
    #[default = true]
    get_gust_jar: bool,
    /// Enter Deepwood Shrine Boss Room
    #[default = true]
    enter_deepwood_shrine_boss_room: bool,
    /// Get Earth Element
    #[default = true]
    get_earth_element: bool,
    /// Enter Mt. Crenel
    #[default = true]
    enter_mt_crenel: bool,
    /// Get Grip Ring
    #[default = true]
    get_grip_ring: bool,
    /// Enter Cave of Flames
    #[default = true]
    enter_cave_of_flames: bool,
    /// Get Cane of Pacci
    #[default = true]
    get_cane_of_pacci: bool,
    /// Enter Cave of Flames Boss Room
    #[default = true]
    enter_cave_of_flames_boss_room: bool,
    /// Get Fire Element
    #[default = true]
    get_fire_element: bool,
    /// Get Pegasus Boots
    #[default = true]
    get_pegasus_boots: bool,
    /// Get Bow
    #[default = true]
    get_bow: bool,
    /// Enter Fortress of Winds
    #[default = true]
    enter_fortress_of_winds: bool,
    /// Get Mole Mitts
    #[default = true]
    get_mole_mitts: bool,
    /// Enter Fortress of Winds Boss Room
    #[default = true]
    enter_fortress_of_winds_boss_room: bool,
    /// Get Ocarina
    #[default = true]
    get_ocarina: bool,
    /// Get Magical Boomerang
    #[default = true]
    get_magical_boomerang: bool,
    /// Get Power Bracelets
    #[default = true]
    get_power_bracelets: bool,
    /// Get Flippers
    #[default = true]
    get_flippers: bool,
    /// Enter Temple of Droplets
    #[default = true]
    enter_temple_of_droplets: bool,
    /// Get Flame Lantern
    #[default = true]
    get_flame_lantern: bool,
    // /// Enter Octo
    // #[default = true]
    // enter_octo: bool,
    /// Get Water Element
    #[default = true]
    get_water_element: bool,
    /// Enter Palace of Winds
    #[default = true]
    enter_palace_of_winds: bool,
    /// Get Roc's Cape
    #[default = true]
    get_rocs_cape: bool,
    // /// Enter Gyorg
    // #[default = true]
    // enter_gyorg: bool,
    /// Get Wind Element
    #[default = true]
    get_wind_element: bool,
    /// Get Four Sword
    #[default = true]
    get_four_sword: bool,
    // /// Enter DHC
    // #[default = true]
    // enter_dhc: bool,
    // /// 2nd Key in DHC
    // #[default = true]
    // second_key_in_dhc: bool,
    // /// Black Knight
    // #[default = true]
    // black_knight: bool,
    /// Get DHC Big Key
    #[default = true]
    get_dhc_big_key: bool,
    // /// Darknuts
    // #[default = true]
    // darknuts: bool,
    // /// Vaati 1
    // #[default = true]
    // vaati_1: bool,
    // /// Vaati 2
    // #[default = true]
    // vaati_2: bool,
    /// Defeat Vaati
    #[default = true]
    defeat_vaati: bool,
}

struct State {
    game: Option<Game>,
    settings: Option<Settings>,
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
    // This is not the target position, it doesn't always update.
    link_position_y: Watcher<u16>,
    visual_rupees: Watcher<u16>,
    visual_hearts: Watcher<u8>,
    visual_keys: Watcher<u8>,
    tiger_scrolls: Watcher<u8>,
    mysterious_shells: Watcher<u16>,
    bombs: Watcher<u8>,
    accumulated_frame_count: i64,
    delayed_split: Option<(&'static str, i64)>,
    run_progress: RunProgress,
}

#[derive(Default)]
struct RunProgress {
    smiths_sword: bool,
    deepwood_shrine: bool,
    deepwood_shrine_boss: bool,
    mt_crenel: bool,
    cave_of_flames: bool,
    cave_of_flames_boss: bool,
    fortress_of_winds: bool,
    fortress_of_winds_boss: bool,
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
            link_position_y: Watcher::new(0x30010BE),
            visual_rupees: Watcher::new(0x200AF0E),
            visual_hearts: Watcher::new(0x200AF03),
            visual_keys: Watcher::new(0x200AF12),
            tiger_scrolls: Watcher::new(0x2002B44),
            mysterious_shells: Watcher::new(0x2002B02),
            bombs: Watcher::new(0x2002AEC),
            accumulated_frame_count: 0,
            delayed_split: None,
            run_progress: Default::default(),
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
            link_position_y: self.link_position_y.update(&self.emulator)?,
            visual_rupees: self.visual_rupees.update(&self.emulator)?,
            visual_hearts: self.visual_hearts.update(&self.emulator)?,
            visual_keys: self.visual_keys.update(&self.emulator)?,
            tiger_scrolls: self.tiger_scrolls.update(&self.emulator)?,
            mysterious_shells: self.mysterious_shells.update(&self.emulator)?,
            bombs: self.bombs.update(&self.emulator)?,
            accumulated_frame_count: &mut self.accumulated_frame_count,
            delayed_split: &mut self.delayed_split,
            run_progress: &mut self.run_progress,
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
    // This is not the target position, it doesn't always update.
    link_position_y: &'a Pair<u16>,
    visual_rupees: &'a Pair<u16>,
    visual_hearts: &'a Pair<u8>,
    visual_keys: &'a Pair<u8>,
    tiger_scrolls: &'a Pair<u8>,
    mysterious_shells: &'a Pair<u16>,
    bombs: &'a Pair<u8>,
    accumulated_frame_count: &'a mut i64,
    delayed_split: &'a mut Option<(&'static str, i64)>,
    run_progress: &'a mut RunProgress,
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

        const MAGICAL_BOOMERANG = 1 << 0;
        const SHIELD = 1 << 2;
        const MIRROR_SHIELD = 1 << 4;
        const FLAME_LANTERN = 1 << 6;

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
    const FORTRESS_OF_WINDS_GREEN_FLOOR: Self = Self(0x58);
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

    pub const MAGICAL_BOOMERANG: usize = 3;
    pub const SHIELD: usize = 3;
    pub const MIRROR_SHIELD: usize = 3;
    pub const FLAME_LANTERN: usize = 3;

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
    let state = &mut *state;
    let settings = state.settings.get_or_insert_with(Settings::register);
    if state.game.is_none() {
        state.game = gba::Emulator::attach().map(Game::new_ntscj);
    }
    if let Some(game) = &mut state.game {
        if !game.emulator.is_open() {
            state.game = None;
            return;
        }
        if let Some(mut vars) = game.update_vars() {
            let mut string = ArrayString::<8>::new();
            let hearts = vars.visual_hearts.current;
            if !(1..=3).contains(&hearts) {
                // Skip the 0 if we show a fraction.
                string.push_str(itoa::Buffer::new().format(hearts / 4));
            }
            match hearts % 4 {
                1 => string.push('¼'),
                2 => string.push('½'),
                3 => string.push('¾'),
                _ => {}
            }
            timer::set_variable("Hearts", &string);
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
                        *vars.run_progress = Default::default();
                        timer::start();
                        timer::pause_game_time();
                    }
                }
                TimerState::Running | TimerState::Paused => {
                    if vars.frame_count.current < vars.frame_count.old {
                        *vars.accumulated_frame_count += vars.frame_count.old as i64 + 1;
                    }

                    timer::set_game_time(frame_count::<60>(vars.frame_count() as u64));

                    if let Some(reason) = should_split(&mut vars, settings) {
                        asr::print_message(reason);
                        timer::split();
                    }
                }
                _ => {}
            }
        }
    }
}

fn should_split(vars: &mut Vars, settings: &Settings) -> Option<&'static str> {
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
        // Workaround to detect loading a save file.
        if vars.run_progress.smiths_sword {
            return None;
        }

        // Get Smith's Sword
        vars.run_progress.smiths_sword = true;
        return settings.get_smiths_sword.then_some("Get Smith's Sword");
    }
    if vars
        .sprite
        .check(|&sprite| sprite == Sprite::RECEIVE_MINISH_CAP)
        && vars.scene.current == Scene::MINISH_WOODS
        && settings.receive_minish_cap
    {
        // Receive Minish Cap
        *vars.delayed_split = Some(("Receive Minish Cap", vars.frame_count() + 20));
        return None;
    }
    if !vars.run_progress.deepwood_shrine
        && vars.scene.current == Scene::DEEPWOOD_SHRINE
        && settings.enter_deepwood_shrine
    {
        // Enter Deepwood Shrine
        vars.run_progress.deepwood_shrine = true;
        return Some("Enter Deepwood Shrine");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::GUST_JAR, InventoryItem::GUST_JAR))
        && settings.get_gust_jar
    {
        // Get Gust Jar
        return Some("Get Gust Jar");
    }
    if !vars.run_progress.deepwood_shrine_boss
        && vars.scene.current == Scene::DEEPWOOD_SHRINE_BOSS
        && settings.enter_deepwood_shrine_boss_room
    {
        // Enter Deepwood Shrine Boss Room
        vars.run_progress.deepwood_shrine_boss = true;
        return Some("Enter Deepwood Shrine Boss Room");
    }
    if vars
        .pause_menu
        .check(|menu| menu.elements.contains(Elements::EARTH))
        && settings.get_earth_element
    {
        // Get Earth Element
        return Some("Get Earth Element");
    }
    if !vars.run_progress.mt_crenel
        && vars.scene.current == Scene::MT_CRENEL
        && settings.enter_mt_crenel
    {
        // Enter Mt. Crenel
        vars.run_progress.mt_crenel = true;
        return Some("Enter Mt. Crenel");
    }
    if vars.pause_menu.check(|menu| {
        menu.permanent_equipment
            .contains(PermanentEquipment::GRIP_RING)
    }) && settings.get_grip_ring
    {
        // Get Grip Ring
        return Some("Get Grip Ring");
    }
    if !vars.run_progress.cave_of_flames
        && vars.scene.current == Scene::CAVE_OF_FLAMES
        && settings.enter_cave_of_flames
    {
        // Enter Cave of Flames
        vars.run_progress.cave_of_flames = true;
        return Some("Enter Cave of Flames");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::CANE_OF_PACCI, InventoryItem::CANE_OF_PACCI))
        && settings.get_cane_of_pacci
    {
        // Get Cane of Pacci
        return Some("Get Cane of Pacci");
    }
    if !vars.run_progress.cave_of_flames_boss
        && vars.scene.current == Scene::CAVE_OF_FLAMES_BOSS
        && settings.enter_cave_of_flames_boss_room
    {
        // Enter Cave of Flames Boss Room
        vars.run_progress.cave_of_flames_boss = true;
        return Some("Enter Cave of Flames Boss Room");
    }
    if vars
        .pause_menu
        .check(|menu| menu.elements.contains(Elements::FIRE))
        && settings.get_fire_element
    {
        // Get Fire Element
        return Some("Get Fire Element");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::PEGASUS_BOOTS, InventoryItem::PEGASUS_BOOTS))
        && settings.get_pegasus_boots
    {
        // Get Pegasus Boots
        return Some("Get Pegasus Boots");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::BOW, InventoryItem::BOW))
        && settings.get_bow
    {
        // Get Bow
        return Some("Get Bow");
    }
    if !vars.run_progress.fortress_of_winds
        && vars.scene.current == Scene::FORTRESS_OF_WINDS
        && settings.enter_fortress_of_winds
    {
        // Enter Fortress of Winds
        vars.run_progress.fortress_of_winds = true;
        return Some("Enter Fortress of Winds");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::MOLE_MITTS, InventoryItem::MOLE_MITTS))
        && settings.get_mole_mitts
    {
        // Get Mole Mitts
        return Some("Get Mole Mitts");
    }
    if !vars.run_progress.fortress_of_winds_boss
        && vars.scene.current == Scene::FORTRESS_OF_WINDS_GREEN_FLOOR
        && vars.link_position_y.current <= 1015
        && settings.enter_fortress_of_winds_boss_room
    {
        // Enter Fortress of Winds Boss Room
        vars.run_progress.fortress_of_winds_boss = true;
        return Some("Enter Fortress of Winds Boss Room");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::OCARINA, InventoryItem::OCARINA))
        && settings.get_ocarina
    {
        // Get Ocarina
        return Some("Get Ocarina");
    }
    if vars.pause_menu.check(|menu| {
        menu.has_item(
            inventory_slot::MAGICAL_BOOMERANG,
            InventoryItem::MAGICAL_BOOMERANG,
        )
    }) && settings.get_magical_boomerang
    {
        // Get Magical Boomerang
        return Some("Get Magical Boomerang");
    }
    if vars.pause_menu.check(|menu| {
        menu.permanent_equipment
            .contains(PermanentEquipment::POWER_BRACELETS)
    }) && settings.get_power_bracelets
    {
        // Get Power Bracelets
        return Some("Get Power Bracelets");
    }
    if vars.pause_menu.check(|menu| {
        menu.permanent_equipment
            .contains(PermanentEquipment::FLIPPERS)
    }) && settings.get_flippers
    {
        // Get Flippers
        return Some("Get Flippers");
    }
    if !vars.run_progress.temple_of_droplets
        && vars.scene.current == Scene::TEMPLE_OF_DROPLETS
        && settings.enter_temple_of_droplets
    {
        // Enter Temple of Droplets
        vars.run_progress.temple_of_droplets = true;
        return Some("Enter Temple of Droplets");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::FLAME_LANTERN, InventoryItem::FLAME_LANTERN))
        && settings.get_flame_lantern
    {
        // Get Flame Lantern
        return Some("Get Flame Lantern");
    }
    // TODO: Enter Octo
    if vars
        .pause_menu
        .check(|menu| menu.elements.contains(Elements::WATER))
        && settings.get_water_element
    {
        // Get Water Element
        return Some("Get Water Element");
    }
    if !vars.run_progress.palace_of_winds
        && vars.scene.current == Scene::PALACE_OF_WINDS
        && settings.enter_palace_of_winds
    {
        // Enter Palace of Winds
        vars.run_progress.palace_of_winds = true;
        return Some("Enter Palace of Winds");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::ROCS_CAPE, InventoryItem::ROCS_CAPE))
        && settings.get_rocs_cape
    {
        // Get Roc's Cape
        return Some("Get Roc's Cape");
    }
    // TODO: Enter Gyorg
    if vars
        .pause_menu
        .check(|menu| menu.elements.contains(Elements::WIND))
        && settings.get_wind_element
    {
        // Get Wind Element
        return Some("Get Wind Element");
    }
    if vars
        .pause_menu
        .check(|menu| menu.has_item(inventory_slot::FOUR_SWORD, InventoryItem::FOUR_SWORD))
        && settings.get_four_sword
    {
        // Get Four Sword
        *vars.delayed_split = Some(("Get Four Sword", vars.frame_count() + 244));
        return None;
    }
    // TODO: Enter DHC
    // TODO: 2nd Key in DHC
    // TODO: Black Knight
    if vars.dhc_big_key.check(|&v| v & 4 != 0) && settings.get_dhc_big_key {
        // Get DHC Big Key
        return Some("Get DHC Big Key");
    }
    // TODO: Darknuts
    // TODO: Vaati 1
    // TODO: Vaati 2
    if vars.scene.current == Scene::VAATI3
        && vars.vaati3_phases.old == 1
        && vars.vaati3_phases.current == 0
        && settings.defeat_vaati
    {
        // Defeat Vaati
        return Some("Defeat Vaati");
    }
    None
}
