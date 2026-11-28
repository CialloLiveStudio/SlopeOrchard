extends Control

const FONT = preload("res://assets/SlopeOrchardSansSC.ttf")
const PINE := Color("#253f3e")
const MOSS := Color("#567862")
const CREAM := Color("#fff1cd")
const LINEN := Color("#ebdfb9")
const INK := Color("#283b40")
const JAM := Color("#a64247")
const GOLD := Color("#edb35f")
const LEVELS := ["t1", "t2", "t5"]
const NAMES := ["初遇黑狼", "节奏压测", "炼金与杂交"]
const GROUPS := ["原生", "强化", "杂交", "伙伴"]
const PAGES := [[0,1,2,3], [4,5,6,7], [8,9,10,11,12,13], [14,15]]
const STOPS := [Vector2(84,114), Vector2(240,96), Vector2(396,106)]
var game: Node
var page := "Title"
var group := 0
var buttons := {}
var test_unlock := false

func open(screen: String) -> void:
	page = screen
	visible = screen != "Game"
	for btn in buttons.values():
		remove_child(btn)
		btn.queue_free()
	buttons.clear()
	match page:
		"Title":
			var resume: bool = game.save.data.started or game.save.data.fruits.size() > 4
			_button("start", "继续游戏" if resume else "开始守卫", Rect2(281,100,145,26), func(): game.start_stage(game.save.continue_stage()), true)
			for spec in [["stages","关卡选择","StageSelect"], ["book","果园图鉴","Almanac"]]:
				_button(spec[0],spec[1],Rect2(281,133 if spec[0] == "stages" else 162,145,24),open.bind(spec[2]))
			_button("sandbox","满配沙盒",Rect2(281,191,145,24),game.start_stage.bind("sandbox"))
		"StageSelect":
			_header_buttons("果园图鉴", "Almanac")
			for i in LEVELS.size():
				var id: String = LEVELS[i]
				var available: bool = game.save.journal.stage_open(id)
				var button := _button(id, NAMES[i], Rect2(24+156*i,193,120,23),
					game.start_stage.bind(id, test_unlock), available)
				button.disabled = not available and not test_unlock
			_button("test", ("[x] " if test_unlock else "[ ] ") + "测试全解锁", Rect2(366,239,102,22), _toggle_test)
		"Almanac":
			_header_buttons("关卡选择", "StageSelect")
			for i in GROUPS.size():
				_button(GROUPS[i],GROUPS[i],Rect2(14 + i * 78,62,72,21), _select_group.bind(i), i == group)
	queue_redraw()

func _button(id: String, words: String, bounds: Rect2, action: Callable, accent := false) -> Button:
	var btn: Button = game._button(bounds.position, bounds.size, words, action, accent, self)
	buttons[id] = btn
	return btn

func _header_buttons(words: String, destination: String) -> void:
	_button("home","主页",Rect2(306,5,43,20),open.bind("Title"))
	_button("other",words,Rect2(353,5,66,20),open.bind(destination))

func _toggle_test() -> void:
	test_unlock = not test_unlock
	open("StageSelect")

func _select_group(index: int) -> void:
	group = index
	open("Almanac")

func _text(at: Vector2, words: String, size: int, color: Color, width := -1, align := HORIZONTAL_ALIGNMENT_LEFT) -> void:
	draw_string(FONT, at, words, align, width, size, color)

func _plate(bounds: Rect2, fill: Color, edge: Color) -> void:
	draw_rect(Rect2(bounds.position + Vector2(2,3), bounds.size), Color("#29393b88"))
	draw_rect(bounds, edge)
	draw_rect(bounds.grow(-2), fill)

func _icon(id: int, at: Vector2, extent: float, locked := false) -> void:
	var tex: Texture2D
	if id == 14:
		tex = game.hound_frames.patrol[0]
	elif id == 15:
		tex = game.lulucy_sprite
	else:
		tex = game.fruit_textures[id]
	var size := (tex.get_size() * extent / maxf(tex.get_width(),tex.get_height())).round()
	draw_texture_rect(tex, Rect2((at-size/2).round(),size),false,Color("#17282a") if locked else Color.WHITE)

func _draw() -> void:
	match page:
		"Title":
			_plate(Rect2(257,34,193,196), CREAM, PINE)
			draw_rect(Rect2(265,41,177,2), MOSS)
			_text(Vector2(271,69), "斜坡果园", 25, PINE, 165, HORIZONTAL_ALIGNMENT_CENTER)
			_text(Vector2(271,88), "SLOPE ORCHARD", 12, JAM, 165, HORIZONTAL_ALIGNMENT_CENTER)
		"StageSelect", "Almanac":
			draw_rect(Rect2(0,0,480,270), CREAM)
			draw_rect(Rect2(0,0,480,31), PINE)
			_text(Vector2(14,21), "关卡选择" if page == "StageSelect" else "果园图鉴", 15, CREAM)
			if page == "StageSelect":
				_draw_route()
			else:
				_draw_book()

func _draw_route() -> void:
	_text(Vector2(17,55), "果园小径", 17, PINE)
	_text(Vector2(320,53), "通关  %d / 3" % game.save.data.clears.size(), 12, MOSS, 145, HORIZONTAL_ALIGNMENT_RIGHT)
	draw_rect(Rect2(0,65,480,76), LINEN)
	var path := PackedVector2Array([Vector2(0,123),STOPS[0],Vector2(133,114),Vector2(158,96),STOPS[1],Vector2(288,96),Vector2(309,106),STOPS[2],Vector2(479,92)])
	draw_polyline(path, MOSS, 5)
	draw_polyline(path, CREAM, 1)
	for i in 3:
		var id: String = LEVELS[i]
		var p: Vector2 = STOPS[i]
		var clear: bool = game.save.data.clears.has(id)
		var available: bool = game.save.journal.stage_open(id)
		var color := PINE if available else Color("#68796e")
		draw_circle(p,23,color)
		draw_circle(p,20,CREAM if available else LINEN)
		_icon([14,1,15][i],p,28,not available)
		_text(Vector2(p.x-34,p.y-24),id.to_upper(),11,color,68,HORIZONTAL_ALIGNMENT_CENTER)
		var x := 18+i*156
		_plate(Rect2(x,145,132,78), Color("#fff7df"), MOSS)
		draw_rect(Rect2(x+5,151,50,14), MOSS if clear else LINEN)
		_text(Vector2(x+5,162),"已通关" if clear else ("待守卫" if available else "未开放"),10,CREAM if clear else INK,50,HORIZONTAL_ALIGNMENT_CENTER)
		var companions: Array = [14,15] if i == 2 else [14]
		for j in companions.size():
			_icon(companions[j], Vector2(x+114-j*24,158),21,not game.save.collected(companions[j]))
		_text(Vector2(x+6,184),"最佳",11,INK)
		for j in 5:
			draw_texture_rect(game.hearts[0 if j < game.save.data.clears.get(id,0) else 1],Rect2(x+39+j*17,173,13,12),false)
	_text(Vector2(17,254),"通关后开放下一站",11,MOSS)

func _draw_book() -> void:
	_text(Vector2(15,53), "果园收藏", 17, PINE)
	_text(Vector2(315,52), "已收集  %d / 16" % (game.save.data.fruits.size()+game.save.data.companions.size()), 12, MOSS, 150, HORIZONTAL_ALIGNMENT_RIGHT)
	var ids: Array = PAGES[group]
	var columns := 3 if ids.size() == 6 else 2
	var width := 145 if columns == 3 else 221
	for i in ids.size():
		var id: int = ids[i]
		var entry: Dictionary = game.save.entries[id]
		var known: bool = game.save.collected(id)
		var at := Vector2(14+(i%columns)*(width+8),92+floori(float(i)/columns)*85)
		_plate(Rect2(at,Vector2(width,77)), Color("#fff8e3") if known else LINEN, MOSS)
		draw_rect(Rect2(at+Vector2(6,8),Vector2(44,44)), Color("#d8e4bd") if known else Color("#a4b4a3"))
		_icon(id,at+Vector2(28,30),33,not known)
		_text(at+Vector2(56,25),entry.name if known else "???",12,PINE)
		if entry.recipe != null:
			_icon(entry.recipe[0],at+Vector2(65,43),16)
			_text(at+Vector2(79,47),"+",11,MOSS)
			_icon(entry.recipe[1],at+Vector2(101,43),16)
		draw_rect(Rect2(at+Vector2(7,56),Vector2(width-14,1)), LINEN if known else MOSS)
		var note: String = entry.note if known else ("相遇后收录" if id >= 14 else "嫁接后收录")
		_text(at+Vector2(9,70), note, 11, INK)

