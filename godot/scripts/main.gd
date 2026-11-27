extends Node2D

const Art = preload("res://scripts/art.gd")
const RetroAudio = preload("res://scripts/retro_audio.gd")
const FONT = preload("res://assets/SlopeOrchardSansSC.ttf")
const SLOTS := [110.0, 165.0, 220.0, 275.0, 330.0, 385.0]
const CLOUDS := [
	Vector3i(14, 70, 31), Vector3i(88, 35, 45), Vector3i(174, 79, 37),
	Vector3i(262, 31, 42), Vector3i(346, 90, 50), Vector3i(415, 54, 48),
]
const HOOK_Y := 62.0
const RESERVE := Vector2(223, 245)
const RESERVE_STEP := 21
const INK := Color("#202b3e")
const PAPER := Color("#fff4d9")
const GOLD := Color("#ffcd72")
const FRUIT_COLORS := [Color("#f4504f"), Color("#a2d45d"), Color("#ffad47")]
const GREEN := Color("#c6eb82")
const SLASH := Color("#ffd27e")
const WAVE := 0
const INTERMISSION := 1
const VICTORY := 2
const DEFEAT := 3
const SANDBOX := 4

var bridge: RefCounted
var model: Dictionary
var background: Texture2D
var clouds: Array[ImageTexture] = []
var fruit_textures: Array[ImageTexture] = []
var green_fruits: Array[ImageTexture] = []
var fruit_flashes: Array[ImageTexture] = []
var monsters: Array[ImageTexture] = []
var monster_flashes: Array[ImageTexture] = []
var bell: ImageTexture
var chief: ImageTexture
var chief_flash: ImageTexture
var hearts: Array[ImageTexture] = []
var audio: Node
var world: Node2D
var hud: Control
var wave_label: Label
var hint: Label
var next_button: Button
var mute_button: Button
var sandbox_button: Button
var elapsed := 0.0
var hitstop := 0.0
var shake := 0.0
var mouse := Vector2(-100, -100)
var trail: Array[Vector2] = []
var trail_life := 0.0
var trail_angled := false
var swiping := false
var swipe_moved := false
var press_at := Vector2.ZERO
var visited := {}
var dragged_item := -1
var drag_button := 0
var drag_item: Dictionary = {}
var flashes := {}
var effects: Array[Dictionary] = []
var current_phase := -1

func _ready() -> void:
	bridge = ClassDB.instantiate("OrchardBridge")
	background = Art.scenery()
	for cloud in CLOUDS:
		clouds.append(Art.cloud(cloud.z))
	for i in Art.FRUIT.size():
		fruit_textures.append(Art.sprite(Art.FRUIT[i], Art.FRUIT_PALETTES[i]))
		green_fruits.append(Art.sprite(Art.FRUIT[i], Art.GREEN_FRUIT_PALETTE))
		fruit_flashes.append(Art.sprite(Art.FRUIT[i], Art.FRUIT_PALETTES[i], true))
	var slime_colors := {"O": "#23483c", "L": "#68c35c", "D": "#388e52", "H": "#c6ed85", "W": "#ffffdc"}
	var boar_colors := {"O": "#302d39", "G": "#8497a0", "B": "#506778", "H": "#c9dade",
		"W": "#fff1c7", "P": "#a47159", "R": "#bb8567", "D": "#74504b"}
	monsters.assign([Art.sprite(Art.SLIME, slime_colors), Art.sprite(Art.BOAR, boar_colors),
		Art.sprite(Art.SHIELD, Art.SHIELD_PALETTE)])
	monster_flashes.assign([Art.sprite(Art.SLIME, slime_colors, true), Art.sprite(Art.BOAR, boar_colors, true),
		Art.sprite(Art.SHIELD, Art.SHIELD_PALETTE, true)])
	bell = Art.sprite(Art.BELL, {"O": "#624238", "G": "#ffd86c", "P": "#f2af53", "D": "#ca7e43", "H": "#fff1ac"})
	chief = Art.sprite(Art.CHIEF, Art.CHIEF_PALETTE)
	chief_flash = Art.sprite(Art.CHIEF, Art.CHIEF_PALETTE, true)
	hearts.assign([
		Art.sprite(Art.HEART, {"O": "#612e40", "R": "#ff625a", "H": "#ffb17a"}),
		Art.sprite(Art.HEART, {"O": "#141e30", "R": "#41516a", "H": "#50617a"}),
	])
	world = Node2D.new()
	world.draw.connect(_paint_world)
	add_child(world)
	audio = RetroAudio.new()
	add_child(audio)
	_build_hud()
	_refresh_model()

func _build_hud() -> void:
	hud = Control.new()
	hud.mouse_filter = Control.MOUSE_FILTER_IGNORE
	hud.draw.connect(_paint_hud)
	add_child(hud)
	wave_label = _label(Vector2(12, 4), 12, "")
	hint = _label(Vector2(12, 246), 11, "")
	sandbox_button = _button(Vector2(284, 3), Vector2(69, 20), "满配沙盒", func(): _restart(true))
	_button(Vector2(358, 3), Vector2(54, 20), "重开", func(): _restart(false))
	mute_button = _button(Vector2(417, 3), Vector2(54, 20), "静音", _toggle_mute)
	next_button = _button(Vector2(393, 234), Vector2(76, 25), "下一波", _continue_run, true)

func _label(at: Vector2, size: int, words: String) -> Label:
	var label := Label.new()
	label.position = at
	label.text = words
	label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	label.add_theme_font_override("font", FONT)
	label.add_theme_font_size_override("font_size", size)
	label.add_theme_color_override("font_color", PAPER)
	hud.add_child(label)
	return label

func _button(at: Vector2, extent: Vector2, words: String, action: Callable, accent := false) -> Button:
	var button := Button.new()
	button.text = words
	button.position = at
	button.focus_mode = Control.FOCUS_NONE
	button.add_theme_font_override("font", FONT)
	button.add_theme_font_size_override("font_size", 11)
	for state in ["normal", "hover", "pressed"]:
		var style := StyleBoxFlat.new()
		style.set_border_width_all(1)
		style.border_color = Color("#d89a5d") if accent else Color("#627b8a")
		if state == "normal":
			style.bg_color = Color("#e8ad69") if accent else Color("#303f54")
		else:
			style.bg_color = Color("#ffdc92") if accent else Color("#496176")
		button.add_theme_stylebox_override(state, style)
	for state in ["font_color", "font_hover_color", "font_pressed_color"]:
		button.add_theme_color_override(state, INK if accent else PAPER)
	button.pressed.connect(action)
	hud.add_child(button)
	button.size = extent
	return button

func _toggle_mute() -> void:
	audio.muted = not audio.muted
	mute_button.text = "开声音" if audio.muted else "静音"

func _restart(sandbox: bool) -> void:
	if sandbox:
		bridge.start_sandbox()
	else:
		bridge.reset_run()
	hitstop = 0
	shake = 0
	effects.clear()
	flashes.clear()
	trail.clear()
	for child in world.get_children():
		child.queue_free()
	_stop_drag()
	swiping = false
	_refresh_model()

func _continue_run() -> void:
	if model.phase == INTERMISSION:
		bridge.next_wave()
		_stop_drag()
		_refresh_model()
	else:
		_restart(false)

func _refresh_model() -> void:
	model = JSON.parse_string(bridge.snapshot())
	var phase := int(model.phase)
	model.phase = phase
	wave_label.text = "沙盒" if phase == SANDBOX else ("第 %d/4 波" % model.wave)
	if current_phase != phase:
		current_phase = phase
		next_button.visible = phase in [INTERMISSION, VICTORY, DEFEAT]
		next_button.text = "下一波" if phase == INTERMISSION else "再来一局"
		match phase:
			INTERMISSION:
				hint.text = "拖动果苗或风铃换位"
			VICTORY:
				hint.text = "胜利 · 果园守住了！"
			DEFEAT:
				hint.text = "失败 · 果园失守"
			SANDBOX:
				hint.text = "左键切果 · 右键拖动"
			_:
				hint.text = "点按或按住左键划过果实"
	hud.queue_redraw()
	world.queue_redraw()

func _process(delta: float) -> void:
	elapsed += delta
	mouse = get_global_mouse_position()
	if not Input.is_mouse_button_pressed(MOUSE_BUTTON_LEFT):
		swiping = false
	if dragged_item >= 0:
		if not Input.is_mouse_button_pressed(drag_button):
			_stop_drag()
	trail_life = maxf(0.0, trail_life - delta)
	if trail_life == 0.0:
		trail.clear()
	shake = maxf(0.0, shake - delta * 17.0)
	world.position = Vector2(round(sin(elapsed * 137.0) * shake), round(cos(elapsed * 171.0) * shake * 0.55))
	for id in flashes.keys():
		if flashes[id] < elapsed:
			flashes.erase(id)
	effects = effects.filter(func(effect: Dictionary): return effect.until > elapsed)
	var frozen := minf(hitstop, delta)
	hitstop -= frozen
	bridge.advance(delta - frozen)
	_refresh_model()
	for event in JSON.parse_string(bridge.drain_events()):
		_effect(event)

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.button_index in [MOUSE_BUTTON_LEFT, MOUSE_BUTTON_RIGHT]:
		mouse = event.position
		if event.pressed:
			if _begin_drag(event.button_index, event.shift_pressed):
				return
			if event.button_index == MOUSE_BUTTON_LEFT and model.phase in [WAVE, SANDBOX]:
				swiping = true
				swipe_moved = false
				press_at = mouse
				visited.clear()
				trail.clear()
		else:
			if event.button_index == drag_button:
				_finish_drag()
			if swiping and event.button_index == MOUSE_BUTTON_LEFT:
				if not swipe_moved:
					_cut_along(press_at, mouse, 0.0)
				swiping = false
	if event is InputEventMouseMotion and swiping:
		var from: Vector2 = event.position - event.relative if swipe_moved else press_at
		if not swipe_moved and from.distance_to(event.position) < 3.0:
			return
		swipe_moved = true
		var vx: float = event.velocity.x
		if is_zero_approx(vx):
			vx = event.relative.x * 60.0
		vx *= 0.12
		if trail.is_empty():
			trail.append(from)
		trail.append(event.position)
		if trail.size() > 8:
			trail.pop_front()
		trail_angled = absf(vx) >= 1.0
		trail_life = 0.09
		_cut_along(from, event.position, vx if trail_angled else 0.0)

func _slot_near(point: Vector2, distance: float) -> int:
	for i in SLOTS.size():
		if point.distance_to(Vector2(SLOTS[i], HOOK_Y)) <= distance:
			return i
	return -1

func _tray_visible() -> bool:
	return model.phase in [INTERMISSION, SANDBOX] or not model.reserve.is_empty()

func _tray_index(point: Vector2, allow_empty: bool) -> int:
	if not _tray_visible() or absf(point.y - RESERVE.y) > 11.0:
		return -1
	var index := roundi((point.x - RESERVE.x) / RESERVE_STEP)
	if absf(point.x - (RESERVE.x + index * RESERVE_STEP)) > RESERVE_STEP * 0.5:
		return -1
	var limit: int = model.reserve.size() + (1 if allow_empty else 0)
	return index + SLOTS.size() if index >= 0 and index < limit else -1

func _begin_drag(button: int, shift: bool) -> bool:
	if model.phase not in [INTERMISSION, SANDBOX]:
		return false
	if model.phase == SANDBOX and button != MOUSE_BUTTON_RIGHT and not shift:
		return false
	var index := _slot_near(mouse, 19)
	if index < 0:
		index = _tray_index(mouse, false)
	if index < 0:
		return false
	var item = model.slots[index] if index < SLOTS.size() else model.reserve[index - SLOTS.size()]
	if item == null:
		return false
	dragged_item = index
	drag_button = button
	drag_item = item
	return true

func _finish_drag() -> void:
	if dragged_item >= 0:
		var target := _slot_near(mouse, 24)
		if target < 0:
			target = _tray_index(mouse, dragged_item < SLOTS.size())
		if target >= 0:
			bridge.swap_plants(dragged_item, target)
	_stop_drag()
	_refresh_model()

func _stop_drag() -> void:
	drag_button = 0
	dragged_item = -1
	drag_item = {}

func _fruit_center(index: int) -> Vector2:
	return Vector2(SLOTS[index] + sin(elapsed * 2.6 + index * 1.2) * 1.5, HOOK_Y)

func _cut_along(from: Vector2, to: Vector2, vx: float) -> void:
	for i in int(model.unlocked_slots):
		var fruit = model.slots[i]
		if fruit == null or fruit.type != "fruit" or not fruit.hanging or visited.has(i):
			continue
		var center := _fruit_center(i) - Vector2(0, 3)
		var closest := Geometry2D.get_closest_point_to_segment(center, from, to) if from != to else to
		if center.distance_to(closest) <= maxf(7.0, 9.0 * fruit.scale):
			var cut: bool = bridge.cut_fruit(i) if is_zero_approx(vx) else bridge.cut_fruit_angled(i, vx)
			if cut:
				visited[i] = true
	_refresh_model()

func _paint_hud() -> void:
	hud.draw_rect(Rect2(0, 0, 480, 26), INK)
	hud.draw_rect(Rect2(0, 26, 480, 2), Color("#466179"))
	for i in 5:
		hud.draw_texture(hearts[0 if i < model.protagonist.hp else 1], Vector2(81 + 14 * i, 7))
	if current_phase in [INTERMISSION, VICTORY, DEFEAT]:
		hud.draw_rect(Rect2(0, 227, 480, 43), INK)
		hud.draw_rect(Rect2(0, 227, 480, 2), GOLD)
	else:
		hud.draw_rect(Rect2(0, 245, int(hint.get_minimum_size().x) + 24, 19), Color("#263146d9"))
	if _tray_visible():
		var size: int = model.reserve.size()
		var empty_target := int(current_phase in [INTERMISSION, SANDBOX] and size < 7)
		hud.draw_rect(Rect2(RESERVE.x - 42, RESERVE.y - 12, 31 + (size + empty_target) * RESERVE_STEP, 25), INK)
		hud.draw_string(FONT, RESERVE + Vector2(-38, 4), "备用", HORIZONTAL_ALIGNMENT_LEFT, -1, 10, GOLD)
		for i in size + empty_target:
			var at := RESERVE + Vector2(i * RESERVE_STEP, 0)
			hud.draw_rect(Rect2(at - Vector2(9, 9), Vector2(19, 19)), Color("#41516a"), false)
			if i >= size or dragged_item == i + SLOTS.size():
				continue
			var item = model.reserve[i]
			var texture := bell if item.type == "chime" else fruit_textures[int(item.kind)]
			hud.draw_texture_rect(texture, Rect2(at - Vector2(8, 8), Vector2(16, 16)), false)

func _stamp(texture: Texture2D, center: Vector2, scale: Vector2 = Vector2.ONE, tint := Color.WHITE) -> void:
	var extent := (texture.get_size() * scale).round()
	world.draw_texture_rect(texture, Rect2((center - extent / 2.0).round(), extent), false, tint)

func _ground(x: float) -> float:
	return 155.0 + 0.194 * (x - 70.0)

func _stamp_item(item: Dictionary, center: Vector2, index: int) -> void:
	if item.type == "chime":
		var cooling := float(item.cooldown) / 120.0
		var wobble := sin(elapsed * (23.0 if cooling > 0.0 else 4.0) + index)
		var tint := Color.WHITE.lerp(Color("#958a72"), cooling * 0.7)
		_stamp(bell, center + Vector2(round(wobble * (1.0 + 2.0 * cooling)), 0), Vector2.ONE, tint)
	elif item.hanging or index < 0:
		var flash: bool = item.stage == 2 and int(item.giant_age) % 104 < 5
		var kind := int(item.kind)
		var texture := green_fruits[kind] if item.stage == 0 else (fruit_flashes[kind] if flash else fruit_textures[kind])
		_stamp(texture, center, Vector2.ONE * float(item.scale))
	else:
		world.draw_rect(Rect2(center.round() - Vector2(1, 2), Vector2(3, 3)), Color("#568e4f"))

func _paint_juice(zone: Dictionary) -> void:
	var first := maxi(0, ceili(zone.left))
	var last := mini(480, floori(zone.right))
	var fade := clampf(float(zone.ticks_left) / 80.0, 0.0, 1.0)
	for x in range(first, last + 1):
		var y := roundf(_ground(x))
		world.draw_rect(Rect2(x, y - 1, 1, 4), Color(0.85, 0.87, 0.37, 0.63 * fade))
		if x % 17 < 5:
			world.draw_rect(Rect2(x, y, 1, 1), Color(1.0, 0.99, 0.69, 0.84 * fade))

func _paint_barricade(barrier: Dictionary) -> void:
	var at := _vec(barrier.body.pos)
	var tint := Color("#837358") if barrier.rotting else Color.WHITE
	_stamp(fruit_textures[2], at, Vector2.ONE * float(barrier.body.scale), tint)
	for i in int(barrier.hits_left):
		world.draw_rect(Rect2(roundf(at.x) - 5 + i * 4, roundf(at.y) - 11, 3, 1), GOLD)
	if barrier.rotting:
		for i in 4:
			var rise := fposmod(float(barrier.age) * 0.1 + i * 4.0, 16.0)
			var bubble := Vector2(at.x + (i - 1.5) * 4 + sin(rise * 0.5) * 2.0, at.y - 6 - rise).round()
			world.draw_rect(Rect2(bubble, Vector2(2, 2)), Color(0.82, 0.85, 0.42, 1.0 - rise / 16.0))

func _paint_world() -> void:
	if model.is_empty():
		return
	world.draw_rect(Rect2(0, 0, 480, 57), Color("#7cc6db"))
	world.draw_rect(Rect2(0, 57, 480, 61), Color("#93d3de"))
	world.draw_rect(Rect2(0, 118, 480, 31), Color("#b7e1dc"))
	world.draw_rect(Rect2(0, 149, 480, 121), Color("#cce8db"))
	for i in CLOUDS.size():
		var cloud: Vector3i = CLOUDS[i]
		var x := fposmod(cloud.x + cloud.z - elapsed * (2.0 + i % 2), 480 + cloud.z) - cloud.z
		world.draw_texture(clouds[i], Vector2(round(x), cloud.y))
	world.draw_texture(background, Vector2.ZERO)
	for zone in model.juice_zones:
		_paint_juice(zone)
	var hero := _vec(model.protagonist.pos)
	var flashing := int(model.protagonist.hit_flash) > 0 and int(model.protagonist.hit_flash) % 8 >= 3
	_stamp(chief_flash if flashing else chief, hero)
	if model.protagonist.swing_cooldown >= 162.0:
		var angle := lerpf(-1.35, 0.45, (180.0 - model.protagonist.swing_cooldown) / 18.0)
		var grip := hero + Vector2(3, 1)
		var tool := grip + Vector2.from_angle(angle) * 16.0
		world.draw_line(grip.round(), tool.round(), Color("#805042"), 2)
		world.draw_rect(Rect2(tool.round() - Vector2(2, 2), Vector2(5, 4)), Color("#fff3ba"))
	var active: bool = model.phase in [WAVE, SANDBOX]
	var focus := _slot_near(mouse, 14.0)
	if active and focus >= 0 and model.slots[focus] != null and model.slots[focus].get("hanging", false):
		var x: float = SLOTS[focus]
		var floor_y := roundi(_ground(x))
		for y in range(77, floor_y, 6):
			world.draw_rect(Rect2(x, y, 1, mini(3, floor_y - y)), Color("#ffffff88"))
		world.draw_rect(Rect2(x - 4, floor_y, 9, 1), Color("#fff8caaa"))
	for i in SLOTS.size():
		var x: float = SLOTS[i]
		var item = model.slots[i]
		if i >= model.unlocked_slots:
			world.draw_rect(Rect2(x - 10, 55, 21, 18), Color("#46586488"))
			for dx in [-8, -3, 2, 7]:
				world.draw_rect(Rect2(x + dx, 58, 2, 13), Color("#536373"))
			world.draw_rect(Rect2(x - 9, 62, 19, 2), Color("#677989"))
			continue
		if dragged_item >= 0:
			world.draw_rect(Rect2(x - 10, HOOK_Y - 8, 21, 20), Color("#fff0a9c8"), false)
		if item == null:
			world.draw_rect(Rect2(x - 1, 59, 3, 3), Color("#446252"))
		elif dragged_item != i:
			var center := _fruit_center(i)
			if item.type == "fruit":
				world.draw_line(Vector2(x, 57), center.round(), Color("#438844"), 1)
				world.draw_rect(Rect2(x - 2, 58, 3, 2), Color("#93ba55"))
			_stamp_item(item, center, i)
	for barrier in model.barricades:
		_paint_barricade(barrier)
	for body in model.bodies:
		var pos := _vec(body.pos)
		var scaling := Vector2.ONE * float(body.scale)
		if body.kind == 1 and body.velocity[1] < -300:
			scaling *= Vector2(0.86, 1.14)
		var texture := green_fruits[int(body.kind)] if body.stage == 0 else fruit_textures[int(body.kind)]
		if body.home_run:
			for i in 3:
				var y := roundf(pos.y) - 3 + i * 3
				world.draw_rect(Rect2(roundf(pos.x) - 23 + i * 3, y, 11 - i * 2, 1), Color("#ffe58bbf"))
		if body.rolling:
			world.draw_set_transform(pos.round(), floorf(pos.x / 6.0) * PI / 8.0, scaling)
			world.draw_texture(texture, -texture.get_size() / 2.0)
			world.draw_set_transform(Vector2.ZERO)
		else:
			_stamp(texture, pos, scaling)
	for enemy in model.monsters:
		var pos := _vec(enemy.pos)
		var struck: bool = flashes.has(enemy.id)
		var squash := Vector2.ONE
		if enemy.kind == 0:
			var lift := _ground(pos.x) - 7.0 - pos.y
			squash = Vector2(1.08 - lift * 0.028, 0.87 + lift * 0.045)
		var texture := monster_flashes[int(enemy.kind)] if struck else monsters[int(enemy.kind)]
		var offset := -1 if enemy.kind == 2 else (2 if enemy.kind == 0 else 3)
		_stamp(texture, pos + Vector2(0, offset), squash)
		var width := 12 if enemy.kind == 0 else 16
		var bar := (pos + Vector2(-width / 2.0, -10 if enemy.kind == 0 else -14)).round()
		world.draw_rect(Rect2(bar, Vector2(width, 3)), INK)
		world.draw_rect(Rect2(bar + Vector2.ONE, Vector2(ceili((width - 2) * enemy.hp / enemy.max_hp), 1)), Color("#ed7958"))
	if dragged_item >= 0:
		_stamp_item(drag_item, mouse, -1)
	if trail.size() > 1:
		var tint := SLASH if trail_angled else PAPER
		tint.a = clampf(trail_life * 15, 0.0, 1.0)
		for i in range(1, trail.size()):
			if trail[i - 1].distance_to(trail[i]) > 0.2:
				world.draw_line(trail[i - 1].round(), trail[i].round(), tint, 1.0 if i < trail.size() - 2 else 2.0)
	for effect in effects:
		var age: float = effect.until - elapsed
		var pos: Vector2 = effect.pos
		if effect.has("text"):
			var at := (pos + Vector2(-3, -25 + age * 26)).round()
			var color: Color = effect.get("color", GOLD if effect.giant else PAPER)
			color.a = clampf(age * 5, 0.0, 1.0)
			world.draw_string_outline(FONT, at, effect.text, HORIZONTAL_ALIGNMENT_LEFT, -1, 13 if effect.giant else 10, 3, INK)
			world.draw_string(FONT, at, effect.text, HORIZONTAL_ALIGNMENT_LEFT, -1, 13 if effect.giant else 10, color)
			if effect.giant:
				world.draw_string(FONT, at + Vector2(1, 0), effect.text, HORIZONTAL_ALIGNMENT_LEFT, -1, 13, color)
		elif effect.has("slash"):
			var vector: Vector2 = effect.slash
			var tint: Color = effect.color
			tint.a = clampf(age * 8.0, 0.0, 1.0)
			world.draw_line((pos - vector * 0.4).round(), (pos + vector).round(), tint, 2.0)
			world.draw_line((pos + vector * 0.64 + Vector2(3, 0)).round(), (pos + vector).round(), tint)
		elif effect.has("reach"):
			var reach: Vector2 = effect.reach
			var tint: Color = effect.color
			tint.a = clampf(age * 6.0, 0.0, 1.0)
			var middle := ((pos + reach) / 2.0 + Vector2(0, -12)).round()
			world.draw_polyline(PackedVector2Array([reach.round(), middle, pos.round()]), tint, 2.0)
			world.draw_rect(Rect2(pos.round() - Vector2(1, 2), Vector2(3, 5)), tint)
		else:
			var radius := roundf(5 + (0.24 - age) * 52.0)
			var color: Color = effect.color
			color.a = clampf(age * 5, 0.0, 1.0)
			world.draw_arc(pos.round(), radius, 0, TAU, 12, color, 1)

func _vec(pair: Array) -> Vector2:
	return Vector2(pair[0], pair[1])

func _effect(event: Array) -> void:
	var pos := _vec(event[1])
	match event[0]:
		"cut":
			_debris(pos, FRUIT_COLORS[int(event[2])], 5)
			var angled := not is_zero_approx(float(event[3]))
			effects.append({"pos": pos, "until": elapsed + 0.15,
				"slash": Vector2(float(event[3]) / 90.0, 1).normalized() * 18.0,
				"color": SLASH if angled else PAPER})
			audio.play("cut")
		"shatter":
			_debris(pos, GREEN if event[3] == 0 else FRUIT_COLORS[int(event[2])], 10)
		"hit":
			for enemy in model.monsters:
				if _vec(enemy.pos).distance_to(pos) < 6.0:
					flashes[enemy.id] = elapsed + 0.095
			hitstop = maxf(hitstop, 0.036 if event[3] else 0.028)
			shake = maxf(shake, 2.6 if event[3] else 1.5)
			effects.append({"pos": pos, "until": elapsed + 0.56, "text": str(int(event[2])), "giant": event[3]})
			_debris(pos, GOLD if event[3] else Color("#fff6d7"), 8)
			audio.play("hit")
		"die":
			_debris(pos, Color("#84ae55"), 13)
		"bounce":
			effects.append({"pos": pos, "until": elapsed + 0.24, "color": Color("#f4ffae")})
			_debris(pos, Color("#c4e584"), 8)
			audio.play("bounce")
		"knock":
			_debris(pos, Color("#c1ec8b"), 7)
		"ring":
			effects.append({"pos": pos, "until": elapsed + 0.24, "color": GOLD})
			_debris(pos, GOLD, 14)
			audio.play("ring")
		"bowl":
			shake = maxf(shake, 1.7)
			_debris(pos, Color("#e4a86d"), 10)
			audio.play("bowl")
		"bite":
			_debris(pos, Color("#efb275"), 5)
			audio.play("bonk")
		"rot":
			_debris(pos, GREEN, 11)
		"chief_hit":
			hitstop = maxf(hitstop, 0.04)
			shake = maxf(shake, 3)
			_debris(pos, Color("#ef795f"), 14)
			audio.play("hit")
		"heal":
			var hero := _vec(model.protagonist.pos)
			effects.append({"pos": hero, "until": elapsed + 0.65,
				"text": "+1", "giant": true, "color": GREEN})
			effects.append({"pos": pos, "until": elapsed + 0.15, "reach": hero + Vector2(7, -6), "color": GREEN})
			_debris(hero - Vector2(0, 8), GREEN, 12)
			audio.play("heal")
		"bonk":
			effects.append({"pos": _vec(model.protagonist.pos), "until": elapsed + 0.38,
				"text": "*", "giant": true})
			_debris(_vec(model.protagonist.pos) - Vector2(0, 6), Color("#c6f1ff"), 8)
			audio.play("bonk")
		"home_run":
			effects.append({"pos": pos, "until": elapsed + 0.18,
				"reach": _vec(model.protagonist.pos) + Vector2(8, -3), "color": SLASH})
			effects.append({"pos": pos, "until": elapsed + 0.24, "color": SLASH})
			shake = maxf(shake, 2.0)
			audio.play("swing")
		"shield":
			effects.append({"pos": pos, "until": elapsed + 0.24, "color": Color("#fff1ba")})
			_debris(pos, GOLD, 10)
			audio.play("shield")

func _debris(at: Vector2, color: Color, count: int) -> void:
	var particles := CPUParticles2D.new()
	particles.position = at
	particles.emitting = false
	particles.one_shot = true
	particles.explosiveness = 1.0
	particles.lifetime = 0.32
	particles.amount = count
	particles.direction = Vector2.UP
	particles.spread = 115
	particles.gravity = Vector2(0, 285)
	particles.initial_velocity_min = 25
	particles.initial_velocity_max = 89
	particles.scale_amount_min = 1.1
	particles.scale_amount_max = 2.6
	particles.color = color
	world.add_child(particles)
	particles.finished.connect(particles.queue_free)
	particles.emitting = true
