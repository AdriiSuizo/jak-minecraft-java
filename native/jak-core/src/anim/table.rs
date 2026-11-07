//! The animations Jak's states play, by the names the art groups give them.
use super::Anim;

pub static NAMES: [&str; 304] = [
    "jakb-attack-from-jump",
    "jakb-attack-from-jump-end",
    "jakb-attack-from-jump-loop",
    "jakb-attack-from-stance",
    "jakb-attack-from-stance-alt-end",
    "jakb-attack-from-stance-end",
    "jakb-attack-from-stance-run-alt-end",
    "jakb-attack-from-stance-run-end",
    "jakb-attack-punch",
    "jakb-attack-punch-alt-end",
    "jakb-attack-punch-end",
    "jakb-attack-uppercut",
    "jakb-blast-recover",
    "jakb-blast-recover-end",
    "jakb-blast-recover-loop",
    "jakb-board-air-turn",
    "jakb-board-airwalk",
    "jakb-board-airwalk-end",
    "jakb-board-airwalk-loop",
    "jakb-board-backgrab",
    "jakb-board-backgrab-end",
    "jakb-board-backgrab-loop",
    "jakb-board-duck-turn",
    "jakb-board-flip-backward",
    "jakb-board-flip-backward-loop",
    "jakb-board-flip-forward",
    "jakb-board-flip-forward-loop",
    "jakb-board-get-off",
    "jakb-board-get-off-pre",
    "jakb-board-get-on",
    "jakb-board-get-on-land",
    "jakb-board-grenade",
    "jakb-board-hit",
    "jakb-board-hit-elec",
    "jakb-board-hit-forward",
    "jakb-board-hit-get-off",
    "jakb-board-jump",
    "jakb-board-jump-high",
    "jakb-board-jump-kick",
    "jakb-board-jump-kickoff",
    "jakb-board-jump-loop",
    "jakb-board-kickflip-a",
    "jakb-board-kickflip-b",
    "jakb-board-kickflip-c",
    "jakb-board-kickspin-a",
    "jakb-board-kickspin-b",
    "jakb-board-kickspin-c",
    "jakb-board-method",
    "jakb-board-method-cross",
    "jakb-board-method-cross-end",
    "jakb-board-method-cross-loop",
    "jakb-board-method-end",
    "jakb-board-method-loop",
    "jakb-board-noseflip",
    "jakb-board-nosegrab",
    "jakb-board-nosegrab-end",
    "jakb-board-nosegrab-loop",
    "jakb-board-ride-turn-back",
    "jakb-board-ride-turn-front",
    "jakb-board-ride-turn-left",
    "jakb-board-ride-turn-right",
    "jakb-board-spin",
    "jakb-board-stance",
    "jakb-board-turn",
    "jakb-board-turn-around",
    "jakb-board-turn-down",
    "jakb-board-turn-left",
    "jakb-board-turn-right",
    "jakb-board-turn-up",
    "jakb-death-lava",
    "jakb-death-painful-land",
    "jakb-deatha",
    "jakb-duck-high-jump",
    "jakb-duck-roll",
    "jakb-duck-roll-end",
    "jakb-duck-stance",
    "jakb-duck-walk",
    "jakb-dummy107",
    "jakb-dummy108",
    "jakb-dummy109",
    "jakb-edge-grab-off",
    "jakb-edge-grab-stance0",
    "jakb-edge-grab-to-jump",
    "jakb-falling-to-edge-grab",
    "jakb-flop-down",
    "jakb-flop-down-land",
    "jakb-flop-down-loop",
    "jakb-flop-jump",
    "jakb-flut-idle",
    "jakb-gun-attack-butt",
    "jakb-gun-attack-butt-blue",
    "jakb-gun-attack-butt-blue-end",
    "jakb-gun-attack-butt-end",
    "jakb-gun-attack-from-stance",
    "jakb-gun-attack-from-stance-blue",
    "jakb-gun-attack-from-stance-blue-end",
    "jakb-gun-attack-from-stance-end",
    "jakb-gun-attack-upperbutt",
    "jakb-gun-attack-upperbutt-blue",
    "jakb-gun-blue-fire",
    "jakb-gun-blue-fire-2",
    "jakb-gun-blue-fire-single",
    "jakb-gun-blue-stance-2",
    "jakb-gun-blue-takeout",
    "jakb-gun-blue-to-front-hop",
    "jakb-gun-blue-to-yellow",
    "jakb-gun-dark-fire",
    "jakb-gun-dark-fire-twirl",
    "jakb-gun-dark-takeout",
    "jakb-gun-duck-roll",
    "jakb-gun-duck-roll-end",
    "jakb-gun-duck-walk",
    "jakb-gun-edge-grab-off",
    "jakb-gun-edge-grab-to-jump",
    "jakb-gun-flop-down",
    "jakb-gun-flop-down-land",
    "jakb-gun-flop-down-loop",
    "jakb-gun-front-jump",
    "jakb-gun-front-jump-land",
    "jakb-gun-front-run",
    "jakb-gun-front-to-blue-hop",
    "jakb-gun-front-to-side-hop",
    "jakb-gun-front-walk",
    "jakb-gun-hit-elec",
    "jakb-gun-hit-from-back",
    "jakb-gun-hit-from-front",
    "jakb-gun-jump-land",
    "jakb-gun-jump-land-side",
    "jakb-gun-red-fire",
    "jakb-gun-red-fire-2",
    "jakb-gun-red-fire-fast",
    "jakb-gun-red-fire-from-sideways",
    "jakb-gun-red-fire-to-sideways",
    "jakb-gun-red-from-sideways",
    "jakb-gun-red-takeout",
    "jakb-gun-roll-flip",
    "jakb-gun-roll-flip-land",
    "jakb-gun-run-blue",
    "jakb-gun-side-jump",
    "jakb-gun-side-jump-land",
    "jakb-gun-side-to-front-hop",
    "jakb-gun-side-to-side-hop-1",
    "jakb-gun-side-to-side-hop-2",
    "jakb-gun-stance",
    "jakb-gun-stance-blue",
    "jakb-gun-stance-dark",
    "jakb-gun-stance-red-sideways",
    "jakb-gun-stance-yellow",
    "jakb-gun-stance-yellow-low",
    "jakb-gun-transformation-twirl",
    "jakb-gun-walk-blue",
    "jakb-gun-walk-side",
    "jakb-gun-yellow-fire",
    "jakb-gun-yellow-fire-3",
    "jakb-gun-yellow-fire-low",
    "jakb-gun-yellow-fire-twirl",
    "jakb-gun-yellow-highlow",
    "jakb-gun-yellow-takeout",
    "jakb-hit-elec",
    "jakb-hit-from-back",
    "jakb-hit-from-front",
    "jakb-hit-from-front-alt1",
    "jakb-hit-up",
    "jakb-invisible-loop",
    "jakb-invisible-to-stance",
    "jakb-jump",
    "jakb-jump-forward",
    "jakb-jump-land",
    "jakb-jump-loop",
    "jakb-jump-short-land",
    "jakb-launch-jump-loop",
    "jakb-lightjak-get-off",
    "jakb-lightjak-get-on",
    "jakb-lightjak-get-on-land",
    "jakb-lightjak-get-on-loop",
    "jakb-lightjak-get-on-out",
    "jakb-lightjak-stance",
    "jakb-lightjak-stance-to-stance",
    "jakb-lightjak-swoop-fall",
    "jakb-lightjak-swoop-fall-loop",
    "jakb-lightjak-swoop-land",
    "jakb-lightjak-swoop1",
    "jakb-lightjak-swoop2",
    "jakb-moving-flop-down",
    "jakb-moving-flop-down-loop",
    "jakb-painful-land",
    "jakb-painful-land-end",
    "jakb-powerjak-get-on",
    "jakb-powerjak-get-on-loop",
    "jakb-roll-flip",
    "jakb-roll-flip-land",
    "jakb-run",
    "jakb-run-down",
    "jakb-run-left",
    "jakb-run-right",
    "jakb-run-squash",
    "jakb-run-squash-weak",
    "jakb-run-to-stance",
    "jakb-run-to-stance-fast",
    "jakb-run-up",
    "jakb-shocked",
    "jakb-smack-surface",
    "jakb-smack-surface-end",
    "jakb-stance-loop",
    "jakb-stance-to-duck",
    "jakb-stance-to-invisible",
    "jakb-trip",
    "jakb-turn-around",
    "jakb-walk",
    "jakb-walk-down",
    "jakb-walk-left",
    "jakb-walk-right",
    "jakb-walk-up",
    "jakb-wall-hide",
    "jakb-wall-hide-body",
    "jakb-wall-hide-head",
    "jakb-wall-hide-head-left",
    "jakb-wall-hide-head-right",
    "jakb-wall-hide-scared",
    "jakb-wall-hide-scared-loop",
    "jakb-wall-hide-scared-return",
    "jakb-wings-lightjak-get-off",
    "jakb-wings-lightjak-get-on-land",
    "jakb-wings-lightjak-stance",
    "jakb-wings-lightjak-swoop-fall",
    "jakb-wings-lightjak-swoop-fall-loop",
    "jakb-wings-lightjak-swoop-land",
    "jakb-wings-lightjak-swoop1",
    "jakb-wings-lightjak-swoop2",
    "jakb-darkjak-get-on-fast",
    "jakb-darkjak-get-off",
    "jakb-darkjak-get-off-end",
    "jakb-darkjak-attack-combo1",
    "jakb-darkjak-attack-combo1b",
    "jakb-darkjak-attack-combo2",
    "jakb-darkjak-attack-combo3c",
    "jakb-wade-shallow-walk",
    "jakb-wade-deep-walk",
    "jakb-swim-stance",
    "jakb-swim-walk",
    "jakb-swim-walk-to-down",
    "jakb-swim-down",
    "jakb-swim-down-to-up",
    "jakb-swim-up",
    "jakb-swim-up-to-stance",
    "jakb-swim-jump",
    // Jak 1's ice (Snowy Mountain), carried onto Jak 3's skeleton by the
    // model loader.
    "jakb-ice-stance",
    "jakb-ice-skate",
    "jakb-ice-slide",
    // Jak 1's riding of his Flut Flut (`eichar-flut-*`), carried onto Jak 3's
    // skeleton by the model loader.
    "jakb1-flut-air-attack",
    "jakb1-flut-air-attack-land",
    "jakb1-flut-air-attack-loop",
    "jakb1-flut-death-drown",
    "jakb1-flut-deatha",
    "jakb1-flut-deathb",
    "jakb1-flut-double-jump",
    "jakb1-flut-get-off",
    "jakb1-flut-get-on",
    "jakb1-flut-hit-back",
    "jakb1-flut-hit-back-end",
    "jakb1-flut-idle",
    "jakb1-flut-jog",
    "jakb1-flut-jump",
    "jakb1-flut-jump-forward",
    "jakb1-flut-jump-forward-land",
    "jakb1-flut-jump-forward-loop",
    "jakb1-flut-jump-land",
    "jakb1-flut-jump-loop",
    "jakb1-flut-run",
    "jakb1-flut-run-squash",
    "jakb1-flut-running-attack",
    "jakb1-flut-running-attack-end",
    "jakb1-flut-smack-surface",
    "jakb1-flut-smack-surface-end",
    "jakb1-flut-squash-run",
    "jakb1-flut-walk",
    "jakb-powerup",
    "jakb-pilot-bike-get-off",
    "jakb-pilot-car-get-off",
    "jakb-lightjak-heal",
    "jakb-lightjak-heal-loop",
    "jakb-lightjak-heal-end",
    "jakb-ladder-get-on","jakb-ladder-stance","jakb-ladder-up","jakb-ladder-down",
    "jakb-edge-grab-to-jump","jakb-ladder-jump","jakb-ladder-slide-loop",
    "jakb-lightjak-freeze","jakb-lightjak-freeze-land","jakb-lightjak-shield","jakb-lightjak-shield-end",
    "jakb-darkjak-bomb-pre","jakb-darkjak-bomb","jakb-darkjak-bomb-loop","jakb-darkjak-bomb-land","jakb-darkjak-bomb-end",
    "jakb-darkjak-attack-ice","jakb-darkjak-attack-ice-loop","jakb-darkjak-attack-ice-loop2","jakb-darkjak-attack-ice-end",
    "jakb1-launch-jump",
    "jakb1-launch-jump-loop",
];

pub const ATTACK_FROM_JUMP: Anim = Anim(0);
pub const ATTACK_FROM_JUMP_END: Anim = Anim(1);
pub const ATTACK_FROM_JUMP_LOOP: Anim = Anim(2);
pub const ATTACK_FROM_STANCE: Anim = Anim(3);
pub const ATTACK_FROM_STANCE_ALT_END: Anim = Anim(4);
pub const ATTACK_FROM_STANCE_END: Anim = Anim(5);
pub const ATTACK_FROM_STANCE_RUN_ALT_END: Anim = Anim(6);
pub const ATTACK_FROM_STANCE_RUN_END: Anim = Anim(7);
pub const ATTACK_PUNCH: Anim = Anim(8);
pub const ATTACK_PUNCH_ALT_END: Anim = Anim(9);
pub const ATTACK_PUNCH_END: Anim = Anim(10);
pub const ATTACK_UPPERCUT: Anim = Anim(11);
pub const BLAST_RECOVER: Anim = Anim(12);
pub const BLAST_RECOVER_END: Anim = Anim(13);
pub const BLAST_RECOVER_LOOP: Anim = Anim(14);
pub const BOARD_AIR_TURN: Anim = Anim(15);
pub const BOARD_AIRWALK: Anim = Anim(16);
pub const BOARD_AIRWALK_END: Anim = Anim(17);
pub const BOARD_AIRWALK_LOOP: Anim = Anim(18);
pub const BOARD_BACKGRAB: Anim = Anim(19);
pub const BOARD_BACKGRAB_END: Anim = Anim(20);
pub const BOARD_BACKGRAB_LOOP: Anim = Anim(21);
pub const BOARD_DUCK_TURN: Anim = Anim(22);
pub const BOARD_FLIP_BACKWARD: Anim = Anim(23);
pub const BOARD_FLIP_BACKWARD_LOOP: Anim = Anim(24);
pub const BOARD_FLIP_FORWARD: Anim = Anim(25);
pub const BOARD_FLIP_FORWARD_LOOP: Anim = Anim(26);
pub const BOARD_GET_OFF: Anim = Anim(27);
pub const BOARD_GET_OFF_PRE: Anim = Anim(28);
pub const BOARD_GET_ON: Anim = Anim(29);
pub const BOARD_GET_ON_LAND: Anim = Anim(30);
pub const BOARD_GRENADE: Anim = Anim(31);
pub const BOARD_HIT: Anim = Anim(32);
pub const BOARD_HIT_ELEC: Anim = Anim(33);
pub const BOARD_HIT_FORWARD: Anim = Anim(34);
pub const BOARD_HIT_GET_OFF: Anim = Anim(35);
pub const BOARD_JUMP: Anim = Anim(36);
pub const BOARD_JUMP_HIGH: Anim = Anim(37);
pub const BOARD_JUMP_KICK: Anim = Anim(38);
pub const BOARD_JUMP_KICKOFF: Anim = Anim(39);
pub const BOARD_JUMP_LOOP: Anim = Anim(40);
pub const BOARD_KICKFLIP_A: Anim = Anim(41);
pub const BOARD_KICKFLIP_B: Anim = Anim(42);
pub const BOARD_KICKFLIP_C: Anim = Anim(43);
pub const BOARD_KICKSPIN_A: Anim = Anim(44);
pub const BOARD_KICKSPIN_B: Anim = Anim(45);
pub const BOARD_KICKSPIN_C: Anim = Anim(46);
pub const BOARD_METHOD: Anim = Anim(47);
pub const BOARD_METHOD_CROSS: Anim = Anim(48);
pub const BOARD_METHOD_CROSS_END: Anim = Anim(49);
pub const BOARD_METHOD_CROSS_LOOP: Anim = Anim(50);
pub const BOARD_METHOD_END: Anim = Anim(51);
pub const BOARD_METHOD_LOOP: Anim = Anim(52);
pub const BOARD_NOSEFLIP: Anim = Anim(53);
pub const BOARD_NOSEGRAB: Anim = Anim(54);
pub const BOARD_NOSEGRAB_END: Anim = Anim(55);
pub const BOARD_NOSEGRAB_LOOP: Anim = Anim(56);
pub const BOARD_RIDE_TURN_BACK: Anim = Anim(57);
pub const BOARD_RIDE_TURN_FRONT: Anim = Anim(58);
pub const BOARD_RIDE_TURN_LEFT: Anim = Anim(59);
pub const BOARD_RIDE_TURN_RIGHT: Anim = Anim(60);
pub const BOARD_SPIN: Anim = Anim(61);
pub const BOARD_STANCE: Anim = Anim(62);
pub const BOARD_TURN: Anim = Anim(63);
pub const BOARD_TURN_AROUND: Anim = Anim(64);
pub const BOARD_TURN_DOWN: Anim = Anim(65);
pub const BOARD_TURN_LEFT: Anim = Anim(66);
pub const BOARD_TURN_RIGHT: Anim = Anim(67);
pub const BOARD_TURN_UP: Anim = Anim(68);
pub const DEATH_LAVA: Anim = Anim(69);
pub const DEATH_PAINFUL_LAND: Anim = Anim(70);
pub const DEATHA: Anim = Anim(71);
pub const DUCK_HIGH_JUMP: Anim = Anim(72);
pub const DUCK_ROLL: Anim = Anim(73);
pub const DUCK_ROLL_END: Anim = Anim(74);
pub const DUCK_STANCE: Anim = Anim(75);
pub const DUCK_WALK: Anim = Anim(76);
pub const DUMMY107: Anim = Anim(77);
pub const DUMMY108: Anim = Anim(78);
pub const DUMMY109: Anim = Anim(79);
pub const EDGE_GRAB_OFF: Anim = Anim(80);
pub const EDGE_GRAB_STANCE0: Anim = Anim(81);
pub const EDGE_GRAB_TO_JUMP: Anim = Anim(82);
pub const FALLING_TO_EDGE_GRAB: Anim = Anim(83);
pub const FLOP_DOWN: Anim = Anim(84);
pub const FLOP_DOWN_LAND: Anim = Anim(85);
pub const FLOP_DOWN_LOOP: Anim = Anim(86);
pub const FLOP_JUMP: Anim = Anim(87);
pub const FLUT_IDLE: Anim = Anim(88);
pub const GUN_ATTACK_BUTT: Anim = Anim(89);
pub const GUN_ATTACK_BUTT_BLUE: Anim = Anim(90);
pub const GUN_ATTACK_BUTT_BLUE_END: Anim = Anim(91);
pub const GUN_ATTACK_BUTT_END: Anim = Anim(92);
pub const GUN_ATTACK_FROM_STANCE: Anim = Anim(93);
pub const GUN_ATTACK_FROM_STANCE_BLUE: Anim = Anim(94);
pub const GUN_ATTACK_FROM_STANCE_BLUE_END: Anim = Anim(95);
pub const GUN_ATTACK_FROM_STANCE_END: Anim = Anim(96);
pub const GUN_ATTACK_UPPERBUTT: Anim = Anim(97);
pub const GUN_ATTACK_UPPERBUTT_BLUE: Anim = Anim(98);
pub const GUN_BLUE_FIRE: Anim = Anim(99);
pub const GUN_BLUE_FIRE_2: Anim = Anim(100);
pub const GUN_BLUE_FIRE_SINGLE: Anim = Anim(101);
pub const GUN_BLUE_STANCE_2: Anim = Anim(102);
pub const GUN_BLUE_TAKEOUT: Anim = Anim(103);
pub const GUN_BLUE_TO_FRONT_HOP: Anim = Anim(104);
pub const GUN_BLUE_TO_YELLOW: Anim = Anim(105);
pub const GUN_DARK_FIRE: Anim = Anim(106);
pub const GUN_DARK_FIRE_TWIRL: Anim = Anim(107);
pub const GUN_DARK_TAKEOUT: Anim = Anim(108);
pub const GUN_DUCK_ROLL: Anim = Anim(109);
pub const GUN_DUCK_ROLL_END: Anim = Anim(110);
pub const GUN_DUCK_WALK: Anim = Anim(111);
pub const GUN_EDGE_GRAB_OFF: Anim = Anim(112);
pub const GUN_EDGE_GRAB_TO_JUMP: Anim = Anim(113);
pub const GUN_FLOP_DOWN: Anim = Anim(114);
pub const GUN_FLOP_DOWN_LAND: Anim = Anim(115);
pub const GUN_FLOP_DOWN_LOOP: Anim = Anim(116);
pub const GUN_FRONT_JUMP: Anim = Anim(117);
pub const GUN_FRONT_JUMP_LAND: Anim = Anim(118);
pub const GUN_FRONT_RUN: Anim = Anim(119);
pub const GUN_FRONT_TO_BLUE_HOP: Anim = Anim(120);
pub const GUN_FRONT_TO_SIDE_HOP: Anim = Anim(121);
pub const GUN_FRONT_WALK: Anim = Anim(122);
pub const GUN_HIT_ELEC: Anim = Anim(123);
pub const GUN_HIT_FROM_BACK: Anim = Anim(124);
pub const GUN_HIT_FROM_FRONT: Anim = Anim(125);
pub const GUN_JUMP_LAND: Anim = Anim(126);
pub const GUN_JUMP_LAND_SIDE: Anim = Anim(127);
pub const GUN_RED_FIRE: Anim = Anim(128);
pub const GUN_RED_FIRE_2: Anim = Anim(129);
pub const GUN_RED_FIRE_FAST: Anim = Anim(130);
pub const GUN_RED_FIRE_FROM_SIDEWAYS: Anim = Anim(131);
pub const GUN_RED_FIRE_TO_SIDEWAYS: Anim = Anim(132);
pub const GUN_RED_FROM_SIDEWAYS: Anim = Anim(133);
pub const GUN_RED_TAKEOUT: Anim = Anim(134);
pub const GUN_ROLL_FLIP: Anim = Anim(135);
pub const GUN_ROLL_FLIP_LAND: Anim = Anim(136);
pub const GUN_RUN_BLUE: Anim = Anim(137);
pub const GUN_SIDE_JUMP: Anim = Anim(138);
pub const GUN_SIDE_JUMP_LAND: Anim = Anim(139);
pub const GUN_SIDE_TO_FRONT_HOP: Anim = Anim(140);
pub const GUN_SIDE_TO_SIDE_HOP_1: Anim = Anim(141);
pub const GUN_SIDE_TO_SIDE_HOP_2: Anim = Anim(142);
pub const GUN_STANCE: Anim = Anim(143);
pub const GUN_STANCE_BLUE: Anim = Anim(144);
pub const GUN_STANCE_DARK: Anim = Anim(145);
pub const GUN_STANCE_RED_SIDEWAYS: Anim = Anim(146);
pub const GUN_STANCE_YELLOW: Anim = Anim(147);
pub const GUN_STANCE_YELLOW_LOW: Anim = Anim(148);
pub const GUN_TRANSFORMATION_TWIRL: Anim = Anim(149);
pub const GUN_WALK_BLUE: Anim = Anim(150);
pub const GUN_WALK_SIDE: Anim = Anim(151);
pub const GUN_YELLOW_FIRE: Anim = Anim(152);
pub const GUN_YELLOW_FIRE_3: Anim = Anim(153);
pub const GUN_YELLOW_FIRE_LOW: Anim = Anim(154);
pub const GUN_YELLOW_FIRE_TWIRL: Anim = Anim(155);
pub const GUN_YELLOW_HIGHLOW: Anim = Anim(156);
pub const GUN_YELLOW_TAKEOUT: Anim = Anim(157);
pub const HIT_ELEC: Anim = Anim(158);
pub const HIT_FROM_BACK: Anim = Anim(159);
pub const HIT_FROM_FRONT: Anim = Anim(160);
pub const HIT_FROM_FRONT_ALT1: Anim = Anim(161);
pub const HIT_UP: Anim = Anim(162);
pub const INVISIBLE_LOOP: Anim = Anim(163);
pub const INVISIBLE_TO_STANCE: Anim = Anim(164);
pub const JUMP: Anim = Anim(165);
pub const JUMP_FORWARD: Anim = Anim(166);
pub const JUMP_LAND: Anim = Anim(167);
pub const JUMP_LOOP: Anim = Anim(168);
pub const JUMP_SHORT_LAND: Anim = Anim(169);
pub const LAUNCH_JUMP_LOOP: Anim = Anim(170);
pub const LIGHTJAK_GET_OFF: Anim = Anim(171);
pub const LIGHTJAK_GET_ON: Anim = Anim(172);
pub const LIGHTJAK_GET_ON_LAND: Anim = Anim(173);
pub const LIGHTJAK_GET_ON_LOOP: Anim = Anim(174);
pub const LIGHTJAK_GET_ON_OUT: Anim = Anim(175);
pub const LIGHTJAK_STANCE: Anim = Anim(176);
pub const LIGHTJAK_STANCE_TO_STANCE: Anim = Anim(177);
pub const LIGHTJAK_SWOOP_FALL: Anim = Anim(178);
pub const LIGHTJAK_SWOOP_FALL_LOOP: Anim = Anim(179);
pub const LIGHTJAK_SWOOP_LAND: Anim = Anim(180);
pub const LIGHTJAK_SWOOP1: Anim = Anim(181);
pub const LIGHTJAK_SWOOP2: Anim = Anim(182);
pub const MOVING_FLOP_DOWN: Anim = Anim(183);
pub const MOVING_FLOP_DOWN_LOOP: Anim = Anim(184);
pub const PAINFUL_LAND: Anim = Anim(185);
pub const PAINFUL_LAND_END: Anim = Anim(186);
pub const POWERJAK_GET_ON: Anim = Anim(187);
pub const POWERJAK_GET_ON_LOOP: Anim = Anim(188);
pub const ROLL_FLIP: Anim = Anim(189);
pub const ROLL_FLIP_LAND: Anim = Anim(190);
pub const RUN: Anim = Anim(191);
pub const RUN_DOWN: Anim = Anim(192);
pub const RUN_LEFT: Anim = Anim(193);
pub const RUN_RIGHT: Anim = Anim(194);
pub const RUN_SQUASH: Anim = Anim(195);
pub const RUN_SQUASH_WEAK: Anim = Anim(196);
pub const RUN_TO_STANCE: Anim = Anim(197);
pub const RUN_TO_STANCE_FAST: Anim = Anim(198);
pub const RUN_UP: Anim = Anim(199);
pub const SHOCKED: Anim = Anim(200);
pub const SMACK_SURFACE: Anim = Anim(201);
pub const SMACK_SURFACE_END: Anim = Anim(202);
pub const STANCE_LOOP: Anim = Anim(203);
pub const STANCE_TO_DUCK: Anim = Anim(204);
pub const STANCE_TO_INVISIBLE: Anim = Anim(205);
pub const TRIP: Anim = Anim(206);
pub const TURN_AROUND: Anim = Anim(207);
pub const WALK: Anim = Anim(208);
pub const WALK_DOWN: Anim = Anim(209);
pub const WALK_LEFT: Anim = Anim(210);
pub const WALK_RIGHT: Anim = Anim(211);
pub const WALK_UP: Anim = Anim(212);
pub const WALL_HIDE: Anim = Anim(213);
pub const WALL_HIDE_BODY: Anim = Anim(214);
pub const WALL_HIDE_HEAD: Anim = Anim(215);
pub const WALL_HIDE_HEAD_LEFT: Anim = Anim(216);
pub const WALL_HIDE_HEAD_RIGHT: Anim = Anim(217);
pub const WALL_HIDE_SCARED: Anim = Anim(218);
pub const WALL_HIDE_SCARED_LOOP: Anim = Anim(219);
pub const WALL_HIDE_SCARED_RETURN: Anim = Anim(220);
pub const WINGS_LIGHTJAK_GET_OFF: Anim = Anim(221);
pub const WINGS_LIGHTJAK_GET_ON_LAND: Anim = Anim(222);
pub const WINGS_LIGHTJAK_STANCE: Anim = Anim(223);
pub const WINGS_LIGHTJAK_SWOOP_FALL: Anim = Anim(224);
pub const WINGS_LIGHTJAK_SWOOP_FALL_LOOP: Anim = Anim(225);
pub const WINGS_LIGHTJAK_SWOOP_LAND: Anim = Anim(226);
pub const WINGS_LIGHTJAK_SWOOP1: Anim = Anim(227);
pub const WINGS_LIGHTJAK_SWOOP2: Anim = Anim(228);
pub const DARKJAK_GET_ON: Anim = Anim(229);
pub const DARKJAK_GET_OFF: Anim = Anim(230);
pub const DARKJAK_GET_OFF_END: Anim = Anim(231);
pub const DARKJAK_COMBO1: Anim = Anim(232);
pub const DARKJAK_COMBO1B: Anim = Anim(233);
pub const DARKJAK_COMBO2: Anim = Anim(234);
pub const DARKJAK_COMBO3: Anim = Anim(235);
pub const WADE_SHALLOW_WALK: Anim = Anim(236);
pub const WADE_DEEP_WALK: Anim = Anim(237);
pub const SWIM_STANCE: Anim = Anim(238);
pub const SWIM_WALK: Anim = Anim(239);
pub const SWIM_WALK_TO_DOWN: Anim = Anim(240);
pub const SWIM_DOWN: Anim = Anim(241);
pub const SWIM_DOWN_TO_UP: Anim = Anim(242);
pub const SWIM_UP: Anim = Anim(243);
pub const SWIM_UP_TO_STANCE: Anim = Anim(244);
pub const SWIM_JUMP: Anim = Anim(245);
pub const ICE_STANCE: Anim = Anim(246);
pub const ICE_SKATE: Anim = Anim(247);
pub const ICE_SLIDE: Anim = Anim(248);
pub const FLUT1_AIR_ATTACK: Anim = Anim(249);
pub const FLUT1_AIR_ATTACK_LAND: Anim = Anim(250);
pub const FLUT1_AIR_ATTACK_LOOP: Anim = Anim(251);
pub const FLUT1_DEATH_DROWN: Anim = Anim(252);
pub const FLUT1_DEATHA: Anim = Anim(253);
pub const FLUT1_DEATHB: Anim = Anim(254);
pub const FLUT1_DOUBLE_JUMP: Anim = Anim(255);
pub const FLUT1_GET_OFF: Anim = Anim(256);
pub const FLUT1_GET_ON: Anim = Anim(257);
pub const FLUT1_HIT_BACK: Anim = Anim(258);
pub const FLUT1_HIT_BACK_END: Anim = Anim(259);
pub const FLUT1_IDLE: Anim = Anim(260);
pub const FLUT1_JOG: Anim = Anim(261);
pub const FLUT1_JUMP: Anim = Anim(262);
pub const FLUT1_JUMP_FORWARD: Anim = Anim(263);
pub const FLUT1_JUMP_FORWARD_LAND: Anim = Anim(264);
pub const FLUT1_JUMP_FORWARD_LOOP: Anim = Anim(265);
pub const FLUT1_JUMP_LAND: Anim = Anim(266);
pub const FLUT1_JUMP_LOOP: Anim = Anim(267);
pub const FLUT1_RUN: Anim = Anim(268);
pub const FLUT1_RUN_SQUASH: Anim = Anim(269);
pub const FLUT1_RUNNING_ATTACK: Anim = Anim(270);
pub const FLUT1_RUNNING_ATTACK_END: Anim = Anim(271);
pub const FLUT1_SMACK_SURFACE: Anim = Anim(272);
pub const FLUT1_SMACK_SURFACE_END: Anim = Anim(273);
pub const FLUT1_SQUASH_RUN: Anim = Anim(274);
pub const FLUT1_WALK: Anim = Anim(275);

pub const POWERUP: Anim = Anim(276);

pub const PILOT_BIKE_GET_OFF: Anim = Anim(277);
pub const PILOT_CAR_GET_OFF: Anim = Anim(278);

/// Light Jak's heal (`target-lightjak-regen`).
pub const LIGHTJAK_HEAL: Anim = Anim(279);
pub const LIGHTJAK_HEAL_LOOP: Anim = Anim(280);
pub const LIGHTJAK_HEAL_END: Anim = Anim(281);

pub const LADDER_GET_ON:Anim=Anim(282);
pub const LADDER_IDLE:Anim=Anim(283);
pub const LADDER_UP:Anim=Anim(284);
pub const LADDER_DOWN:Anim=Anim(285);
pub const LADDER_GET_OFF:Anim=Anim(286);
pub const LADDER_JUMP:Anim=Anim(287);
pub const LADDER_SLIDE:Anim=Anim(288);

/// Light Jak's clap (`target-lightjak-freeze`) and his shield's move (`target-lightjak-shield`).
pub const LIGHTJAK_FREEZE: Anim = Anim(289);
pub const LIGHTJAK_FREEZE_LAND: Anim = Anim(290);
pub const LIGHTJAK_SHIELD: Anim = Anim(291);
pub const LIGHTJAK_SHIELD_END: Anim = Anim(292);
/// The Dark Bomb (`target-darkjak-bomb0`) and the Dark Blast (`target-darkjak-bomb1`, out of the dark uppercut).
pub const DARKJAK_BOMB_PRE: Anim = Anim(293);
pub const DARKJAK_BOMB: Anim = Anim(294);
pub const DARKJAK_BOMB_LOOP: Anim = Anim(295);
pub const DARKJAK_BOMB_LAND: Anim = Anim(296);
pub const DARKJAK_BOMB_END: Anim = Anim(297);
pub const DARKJAK_ATTACK_ICE: Anim = Anim(298);
pub const DARKJAK_ATTACK_ICE_LOOP: Anim = Anim(299);
pub const DARKJAK_ATTACK_ICE_LOOP2: Anim = Anim(300);
pub const DARKJAK_ATTACK_ICE_END: Anim = Anim(301);
pub const LAUNCH1_JUMP: Anim = Anim(302);
pub const LAUNCH1_JUMP_LOOP: Anim = Anim(303);
