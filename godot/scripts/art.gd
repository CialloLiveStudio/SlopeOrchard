extends RefCounted

const INK := "#243044"
const FRUIT := [
	[
		".........LL.....", "........LllL....", ".......LllL.....", ".......TLL......",
		"....OO.T.OO.....", "...ORROORRRO....", "..ORHRRRRRRRO...", ".ORHHRRRRRRRRO..",
		".ORHRRRRRRRDRO..", ".ORRRRRRRRRDDO..", ".ORRRRRRRRDDDO..", "..ORRRRRRDDDO...",
		"..ORRRRRDDDDO...", "...ODRDDDDDO....", "....OOODDOO.....", ".......OO.......",
	],
	[
		"................", "........T.......", ".......TlL......", ".....OOOTOL.....",
		"...OOLHLoLOO....", "..OLlHloLLlLO...", ".OLlHloLLloLLO..", ".OLlHloLLlolloO.",
		"OLlHloLLlolloLO.", "OLllloLLlolloLO.", "OLllloLLlolloLO.", ".OLllloLLoloLO..",
		".OLllloLooloLO..", "..OOLlloLloOO...", "....OOLOLOO.....", "......OOO.......",
	],
	[
		"........................", "........................", "............LL..........", "............TlL.........",
		"............TLL.........", "...........TT...........", "......OO..OTOO.OO.......", "....OOPPOOPPPOOPPOO.....",
		"...OPPPPOPPHPPOPPPDO....", "..OPPHPODPPHPPODPPDDO...", ".OPPHPPODPPHPPODPPDDDO..",
		".OPPHPPODPPHPPODPPDDDO..", ".OPPPPPODPPHPPODPPPDDO..", ".OPPPPPODPPPPPODPPPDDO..",
		".ODPPPPODPPPPPODPPDDDO..", "..ODPPPODPPPPPODPDDDO...", "..ODDPPODDPPPPODDDDDO...",
		"...ODDDODDDPDDODDDDO....", "....OOODODDDDOODOOO.....", "......OOOODDOOOO........",
		"..........OOO...........", "........................", "........................", "........................",
	],
	[
		"......LlL.....", "...LLLllLL....", "....LLTLL.....", "...OOO.OOO....",
		"..OPPHOPPHO...", ".OPPHPOPPPDO..", ".OPPPDOOPPDO..", "..ODDOPOODO...",
		"...OPPHPOPO...", "...OPPPPDDO...", "....ODDDDO....", ".....OPPO.....",
		"......OO......", "..............",
	],
]
const FRUIT_PALETTES := [
	{"O": "#522639", "L": "#22744b", "l": "#96cf52", "T": "#77523a", "R": "#f4504f", "D": "#ce3548", "H": "#ffc095"},
	{"O": "#23493d", "L": "#489448", "l": "#a2d45d", "T": "#77523a", "H": "#d9eb8b", "o": "#286d49"},
	{"O": "#633b31", "L": "#3b7f42", "l": "#93ca59", "T": "#70502e", "P": "#ffad47", "D": "#e27232", "H": "#ffe498"},
	{"O": "#39314a", "L": "#42983e", "l": "#d4f882", "T": "#785b50", "P": "#ad59bf", "D": "#693f91", "H": "#ffc1eb"},
]
const LULUCY := [
	"....O.....O.....", "...OVO...OVO....", "...OVVOOOVVO....", "....OVVVVVO.....",
	"...OVHHHHVVO....", "...OHREERHHO....", "....OEFFEO......", "...OVOEEOVO.....",
	"..OVVVRRVVVO....", ".OVVVORROVVVO...", "OVVVOORROOVVVO..", "OOOVORRRROVOOO..",
	"..OORWRRWROO....", "...ORRRRRRO.....", "..ORRRRRRRRO....", ".OOOOOOOOOOOO...",
	"....OB..BO......", "...OOO..OOO.....",
]
const ALCHEMIST_PALETTE := {"O":"#30253d","V":"#8c54a6","H":"#dfc5ed","R":"#9c3e68","E":"#f3c7ae","F":"#fff6df","W":"#edab83","B":"#534157"}
const SLIME := [
	"................", "................", ".....OOOOO......", "...OOLLLHOO.....",
	"..OLLLHHLLLO....", "..OLLHLLLLLLO...", ".OLLLOOLLOLLLO..", ".OLLLOWLLOWLLO..",
	".OLLLOOLLOLLLO..", "OLLLLLLLLLLLLLO.", "OLLLDDLDDDDLLLO.", "OLDDDDDDDDDDDDO.",
	".OOOOOOOOOOOOO..", "................", "................", "................",
]
const BOAR := [
	"........................", "........................", "......OOOO..............", "....OOGHGGOO............",
	"...OGHGGGGGBO....OOOO...", "..OGHGGGGBBBOOOOORRROO..", "..OGGGGBBBBBOPPPPPDDDOO.",
	"..OOGBBBBOOOPPPPPPPDDDO.", ".OROOBOOOWORRRPPPPPDDDO.", "ORRROOWOROORRRPPPPDDDDO.",
	"ORHROOOORROORRRPPDDDDDO.", "ORROOWORRRRODDDRDDDDDDO.", ".OOOWWORRRDDDDDDDDDDDO..",
	"..OOOORRDDDDDDDDDDDDO...", "....OOOODDDOOOOOODDO....", "......ODDOOO...ODDOO....",
	"......ODOO.....ODDO.....", "......OOOO.....OOOO.....", "........................", "........................",
	"........................", "........................", "........................", "........................",
]
const SHIELD := [
	"....................", "....................", "....................",
	"......OOOOOOO.......", ".....OVVVVVVVO......", "....OOOVVVVOOOO.....",
	".....OVVVVVVVO......", "....OVWVOVVVO.......", "....OVVVVVVVVO......",
	"....OOVVVVVVOO......", "...OOVBBVBBVOOO.....", "...OVBBBBBBBVO......",
	"...OOVBBBBBOOO......", ".....OBBBOOKO.......", "....OKKOOOKKO.......",
	"....OOOO.OOOO.......",
]
const SHIELD_PALETTE := {
	"O": "#422f3f", "V": "#9b777e", "W": "#fff9d6", "B": "#657985", "K": "#4d495b",
}
const POT_LIDS := [
	[
		".........OOO........", "...OOOOOOHDDOOOOO...",
		"..OHYYYYHHHYYYYYDO..", ".OHYYYYYYYYYYYYYYDO.",
		".OYYYYYYYYYYDDDDDDO.", "..ODDDDDDDDDDDDDOO..",
	],
	[
		".........OOO........", "...OOOOOOHDODDOOO...",
		"..OHYYYYHHHODDYYDO..", ".OHYYYYYYYYDYYYYYDO.",
		".OYYYYYYYYDDDDDDDO.", "..ODDDDDDDDDDDDDOO..",
	],
	[
		"........OOO.........", "...OOOOOHDODO....O..",
		"..OHYYYYHDOODOOOODO.", ".OHYYYYYDOODYYYYDDO.",
		".OYYYYYYDDDDDDDDDO.", "..ODDDDDDDDDDDDDOO..",
	],
]
const LID_PALETTE := {"O": "#422f3f", "H": "#fff3ba", "Y": "#ffc965", "D": "#b66c3f"}
const SWEAT := ["..O..", ".OHO.", ".OHBO", "OHBBO", ".OOO."]
const SWEAT_PALETTE := {"O": "#376584", "H": "#e5faff", "B": "#7bdaed"}
const CHIEF := [
	".......OOO..........", "......OYYYO.........", ".....OYYYYDO........",
	"...OOYYYYYYDO.......", "..OHHHHHHHDDDDO.....", "...OOOSSSSOOO.......",
	".....OSOSOSO.....KK.", ".....OSSWSWO....KHK.", "......OWWWO.....KK..",
	".....OOBBBOO...T....", "....OBGBBBBOO.T.....", "....OBBBBBBOST......",
	"....OBBBBBOTTO......", ".....OBBBBOO........", ".....OBBBBO.........",
	".....OBBOBO.........", ".....OTTOOTO........", "....OTTOOTTO........",
	"....OOOOOOOO........",
]
const CHIEF_PALETTE := {
	"O": "#3f3040", "H": "#ffebb0", "Y": "#e8ad52", "D": "#af653d",
	"S": "#f2b58a", "W": "#fff2d2", "B": "#498a92", "G": "#8fc7b1",
	"T": "#805042", "K": "#39465b",
}
const GREEN_FRUIT_PALETTE := {
	"O": "#284934", "L": "#498144", "l": "#9bc760", "T": "#77523a",
	"R": "#83b64b", "P": "#83b64b", "D": "#4f8c47", "H": "#d5e889", "o": "#2d6c48",
}
const BELL := [
	".......OO.......", ".......OGO......", ".......OO.......", "......OGGO......",
	".....OHGGGO.....", ".....OHGPGO.....", "....OHGGPGDO....", "....OGGGPGDO....",
	"...OGGGGGDDO....", "...OHHHGGGDDO...", "..OOOOOOOOOOO...", "......ODO.......",
	"......OGO.......", ".......O........", "................", "................",
]
const HEART := [
	"............", ".OOO...OOO..", "ORRRO.ORRRO.", "ORHRRORRRRO.",
	"ORRRRRRRRRO.", ".ORRRRRRRO..", "..ORRRRRO...", "...ORRRO....",
	"....ORO.....", ".....O......",
]
const HELLHOUND := [
	"..............O..O...", ".............OBOOBO..",
	"............OBPBBBO..", "......OOOOOOOBHHBBO..",
	"...OOOBBBBBBBBHBYBO..", "..OBBBHHBBBBBBDBBBOO.",
	"..OBBBBBBBBBBDDBBBDBO", "..OBDDDDDDDBDDDOOOOO.",
	"...OOOOOOOOOOOO......",
]
const HELLHOUND_SIT := [
	"............O..O.....", "...........OBOOBO....",
	"..........OBPBBBO....", "..........OBHHBBO....",
	".........OBHBYBOO....", ".......OOBBDBBBDBO...",
	".....OOBBBDDOOOOO....", "....OBBBBDDDO........",
	"...OBBHBDDDOBO.......", "...OBBBDDDOBBO.......",
	"...OOOOOOOOOOO.......",
]
const HELLHOUND_NAP := [
	"..........O..O.......", "....OOOOOOBOOBO......",
	"..OOBBBBBBBHBBO......", ".OBBHHBBBDDBDBOO.....",
	".OBBBBBBDDDDBBDBO....", "..OOOOOOOOOOOOOO.....",
]
const HOUND_PALETTE := {
	"O": "#202436", "B": "#445067", "D": "#30364c",
	"H": "#69788c", "P": "#977987", "Y": "#ffd679",
}

static func sprite(rows: Array, colors: Dictionary, white: bool = false) -> ImageTexture:
	var bitmap := Image.create_empty(rows[0].length(), rows.size(), false, Image.FORMAT_RGBA8)
	_pixels(bitmap, rows, colors, Vector2i.ZERO, white)
	return ImageTexture.create_from_image(bitmap)

static func _pixels(bitmap: Image, rows: Array, colors: Dictionary, offset: Vector2i, white := false) -> void:
	for y in rows.size():
		for x in rows[y].length():
			var pixel: String = rows[y][x]
			if colors.has(pixel):
				bitmap.set_pixel(x + offset.x, y + offset.y, Color.WHITE if white else Color(colors[pixel]))

static func hellhound(pose: String, frame: int = 0) -> ImageTexture:
	var bitmap := Image.create_empty(29, 21, false, Image.FORMAT_RGBA8)
	if pose == "nap":
		_pixels(bitmap, HELLHOUND_NAP, HOUND_PALETTE, Vector2i(5, 13))
	elif pose == "tamed":
		var wag := 2 * (frame % 2)
		box(bitmap, 2, 14 - wag, 4, 3 + wag, HOUND_PALETTE.O)
		box(bitmap, 4, 16, 7, 3, HOUND_PALETTE.O)
		box(bitmap, 3, 15 - wag, 2, 2 + wag, HOUND_PALETTE.B)
		_pixels(bitmap, HELLHOUND_SIT, HOUND_PALETTE, Vector2i(5, 9))
	else:
		var charging := pose == "charge"
		var y := 9 if charging else 7
		var walking := pose in ["patrol", "charge"]
		var stride := (-2 if frame == 0 else 2) if walking else 0
		box(bitmap, 0, y + 1, 4, 2, HOUND_PALETTE.O)
		box(bitmap, 2, y + 2, 4, 3, HOUND_PALETTE.O)
		box(bitmap, 4, y + 3, 7, 3, HOUND_PALETTE.O)
		box(bitmap, 2, y + 2, 7, 1, HOUND_PALETTE.B)
		for i in 4:
			var leg_x := 10 + i * 2 if i < 2 else 16 + i * 2
			var foot_x := leg_x + (stride if i % 2 == 0 else -stride)
			var foot_y := 16 if pose == "patrol" and i % 2 != frame else 17
			box(bitmap, leg_x, y + 6, 2, foot_y - y - 4, HOUND_PALETTE.O)
			box(bitmap, mini(leg_x, foot_x), foot_y, absi(leg_x - foot_x) + 2, 2, HOUND_PALETTE.O)
			box(bitmap, foot_x, foot_y + 1, 2, 1, HOUND_PALETTE.H if i % 2 == 0 else HOUND_PALETTE.B)
		_pixels(bitmap, HELLHOUND, HOUND_PALETTE, Vector2i(6 if charging else 5, y))
		match pose:
			"munch":
				box(bitmap, 21, y + 4, 3, 1, HOUND_PALETTE.O)
				box(bitmap, 24, y + 5, 2, 1, "#f397ad")
				box(bitmap, 24, y + 7, 3, 1 + frame, HOUND_PALETTE.O)
				if frame == 1:
					box(bitmap, 25, y + 8, 1, 1, "#f397ad")
			"pant":
				box(bitmap, 24, y + 7, 3, 1, HOUND_PALETTE.O)
				box(bitmap, 25, y + 8, 2, 1 + frame, "#d77791")
			"sour":
				box(bitmap, 21, y + 4, 3, 1, HOUND_PALETTE.O)
				box(bitmap, 24, y + 7, 3, 1, "#c8d079")
	return ImageTexture.create_from_image(bitmap)

static func scenery(slots: Array) -> ImageTexture:
	var art := Image.create_empty(480, 270, false, Image.FORMAT_RGBA8)
	var rng := RandomNumberGenerator.new()
	rng.seed = 7291
	var ground_rows := PackedInt32Array()
	for x in 480:
		var y := roundi(155.0 + 0.194 * (x - 70.0))
		ground_rows.append(y)
		var ridge := roundi(160 + sin(x * 0.021) * 9 + cos(x * 0.012) * 12)
		box(art, x, ridge, 1, 270 - ridge, "#9dcbb1")
		var hill := roundi(183 + cos(x * 0.017 + 2.0) * 11 + sin(x * 0.008) * 8)
		box(art, x, hill, 1, 270 - hill, "#7eae94")
	for x in range(67, 480, 17):
		var y := roundi(188 + cos(x * 0.017 + 2.0) * 11 + sin(x * 0.008) * 8)
		box(art, x + 3, y, 2, 5, "#649781")
		box(art, x, y - 8, 9, 9, "#6c9f89")
		box(art, x + 2, y - 10, 5, 3, "#6c9f89")
	for x in 480:
		var y := ground_rows[x]
		box(art, x, y, 1, 270 - y, "#704738")
		box(art, x, y + 7, 1, 28, "#966044")
		box(art, x, y + 9, 1, 11, "#a9704b")
		box(art, x, y, 1, 7, "#39764d")
		box(art, x, y + 1, 1, 3, "#77ad55")
	for i in 580:
		var x := rng.randi_range(0, 478)
		var depth := rng.randi_range(9, 120)
		var y := ground_rows[x] + depth
		if y < 269:
			var color := "#b77d56" if depth < 33 else "#845640"
			if i % 3 == 0:
				color = "#623c34"
			box(art, x, y, rng.randi_range(1, 4), 1, color)
	for x in range(5, 480, 9):
		var y := ground_rows[x]
		box(art, x, y - 2, 2, 4, "#77ad55")
		if x % 4 == 0:
			box(art, x + 1, y - 4, 1, 3, "#39764d")
	cabin(art)
	trellis(art, slots)
	return ImageTexture.create_from_image(art)

static func cloud(width: int) -> ImageTexture:
	var art := Image.create_empty(width, 17, false, Image.FORMAT_RGBA8)
	box(art, 0, 10, width, 5, "#eaf7e7")
	box(art, 7, 4, width - 15, 6, "#f5fae9")
	box(art, 13, 0, width / 3, 4, "#fffbed")
	box(art, 4, 15, width - 10, 2, "#d5efe8")
	return ImageTexture.create_from_image(art)

static func box(art: Image, x: int, y: int, w: int, h: int, color: String) -> void:
	art.fill_rect(Rect2i(x, y, w, h), Color(color))

static func cabin(art: Image) -> void:
	box(art, 12, 144, 48, 3, "#3b533d")
	box(art, 15, 119, 42, 26, INK)
	box(art, 17, 121, 38, 21, "#c88d53")
	for y in range(123, 142, 4):
		box(art, 17, y, 38, 1, "#a66a48")
	box(art, 42, 122, 13, 23, "#4b3438")
	box(art, 44, 124, 9, 19, "#805542")
	box(art, 45, 125, 1, 16, "#ab7852")
	box(art, 50, 133, 2, 2, "#ffcd75")
	box(art, 42, 144, 18, 3, "#927a65")
	box(art, 48, 147, 13, 2, "#796152")
	box(art, 22, 123, 13, 12, INK)
	box(art, 24, 125, 9, 8, "#ffbf59")
	box(art, 24, 125, 5, 4, "#ffe9a3")
	box(art, 28, 125, 1, 8, "#99563c")
	box(art, 24, 129, 9, 1, "#99563c")
	box(art, 45, 96, 8, 14, "#573e43")
	box(art, 46, 97, 5, 12, "#b46350")
	box(art, 44, 95, 10, 3, "#824652")
	for y in range(100, 121):
		var half_width := y - 99 + 5
		box(art, 35 - half_width, y, half_width * 2, 1, "#592e40")
		box(art, 37 - half_width, y, half_width * 2 - 4, 1, "#d85d52" if y % 4 < 2 else "#b84747")
	box(art, 9, 120, 53, 3, "#59353e")
	box(art, 12, 120, 46, 1, "#f08461")

static func trellis(art: Image, slots: Array) -> void:
	var left := int(slots.front()) - 29
	var right := int(slots.back()) + 22
	for x in [left + 7, right]:
		box(art, x, 34, 5, 16, "#5a3c34")
		box(art, x + 1, 35, 2, 14, "#c99159")
	var width := right - left + 12
	box(art, left, 36, width, 7, "#5a3c34")
	box(art, left + 1, 37, width - 2, 2, "#e6ad6a")
	box(art, left + 2, 39, width - 3, 2, "#b77a4e")
	for slot in slots:
		var x := int(slot)
		box(art, x - 1, 41, 3, 5, "#64463a")
		box(art, x - 2, 42, 5, 4, "#f3bd72")
		box(art, x, 44, 1, 3, "#3c6042")

