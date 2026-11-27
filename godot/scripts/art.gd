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
]
const FRUIT_PALETTES := [
	{"O": "#522639", "L": "#22744b", "l": "#96cf52", "T": "#77523a", "R": "#f4504f", "D": "#ce3548", "H": "#ffc095"},
	{"O": "#23493d", "L": "#489448", "l": "#a2d45d", "T": "#77523a", "H": "#d9eb8b", "o": "#286d49"},
	{"O": "#633b31", "L": "#3b7f42", "l": "#93ca59", "T": "#70502e", "P": "#ffad47", "D": "#e27232", "H": "#ffe498"},
]
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

static func sprite(rows: Array, colors: Dictionary, white: bool = false) -> ImageTexture:
	var bitmap := Image.create_empty(rows[0].length(), rows.size(), false, Image.FORMAT_RGBA8)
	for y in rows.size():
		for x in rows[y].length():
			var pixel: String = rows[y][x]
			if colors.has(pixel):
				bitmap.set_pixel(x, y, Color.WHITE if white else Color(colors[pixel]))
	return ImageTexture.create_from_image(bitmap)

static func scenery() -> ImageTexture:
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
	trellis(art)
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

static func trellis(art: Image) -> void:
	for x in [89, 407]:
		box(art, x, 48, 5, 16, "#5a3c34")
		box(art, x + 1, 49, 2, 14, "#c99159")
	box(art, 82, 50, 337, 7, "#5a3c34")
	box(art, 83, 51, 335, 2, "#e6ad6a")
	box(art, 84, 53, 334, 2, "#b77a4e")
	for x in [110, 165, 220, 275, 330, 385]:
		box(art, x - 2, 52, 5, 6, "#64463a")
		box(art, x - 1, 53, 3, 3, "#f3bd72")
		box(art, x, 57, 1, 2, "#3c6042")

