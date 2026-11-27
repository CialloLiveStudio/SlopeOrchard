extends Node2D

const Art = preload("res://scripts/art.gd")
const RetroAudio = preload("res://scripts/retro_audio.gd")
const FONT = preload("res://assets/SlopeOrchardSansSC.ttf")
const CLOUDS := [
	Vector3i(14, 70, 31), Vector3i(88, 35, 45), Vector3i(174, 79, 37),
	Vector3i(262, 31, 42), Vector3i(346, 90, 50), Vector3i(415, 54, 48),
]
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
const DASH_VFX_TIME := 0.23
const COMBO_HITSTOP := 0.020

var bridge: RefCounted
var model: Dictionary
var slot_x: Array
var background: Texture2D
var clouds: Array[ImageTexture] = []
var fruit_textures: Array[ImageTexture] = []
var green_fruits: Array[ImageTexture] = []
var fruit_flashes: Array[ImageTexture] = []
var monsters: Array[ImageTexture] = []
var monster_flashes: Array[ImageTexture] = []
var hound_frames := {}
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
var companion_button: Button
var level_buttons := {}
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
var dragged_item := -1
var drag_button := 0
var drag_item: Dictionary = {}
var flashes := {}
var effects: Array[Dictionary] = []
var dash: Dictionary = {}
var current_phase := -1
var hound_charge_until := 0.0
var chief_bonk_until := 0.0

func _ready() -> void:
	bridge = ClassDB.instantiate("OrchardBridge")
	var layout: Dictionary = JSON.parse_string(bridge.snapshot())
	slot_x = layout.slot_x
	background = Art.scenery(slot_x)
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
	for pose in ["patrol", "charge", "tamed", "nap"]:
		hound_frames[pose] = [Art.hellhound(pose, 0), Art.hellhound(pose, 1)]
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
	wave_label = _label(Vector2(87, 5), 11, "")
	hint = _label(Vector2(12, 246), 11, "")
	level_buttons["t1"] = _button(Vector2(153, 3), Vector2(76, 20), "T1 初遇黑狼", func(): _restart("t1"))
	level_buttons["t2"] = _button(Vector2(233, 3), Vector2(76, 20), "T2 节奏压测", func(): _restart("t2"))
	level_buttons["sandbox"] = _button(Vector2(313, 3), Vector2(60, 20), "满配沙盒", func(): _restart("sandbox"))
	_button(Vector2(377, 3), Vector2(42, 20), "重开", func(): _restart())
	mute_button = _button(Vector2(423, 3), Vector2(48, 20), "静音", _toggle_mute)
	companion_button = _button(Vector2(383, 234), Vector2(88, 25), "黑狼出战: 开", _toggle_companion)
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
	for state in ["normal", "hover", "pressed", "hover_pressed"]:
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

func _toggle_companion() -> void:
	if bridge.set_sandbox_companion(not model.hellhound.sandbox_enabled):
		hound_charge_until = 0
		_refresh_model()

func _restart(level_id := "") -> void:
	if level_id.is_empty():
		bridge.reset_run()
	else:
		bridge.start_level(level_id)
	_reset_presentation()

func _reset_presentation() -> void:
	hitstop = 0
	shake = 0
	hound_charge_until = 0
	chief_bonk_until = 0
	world.position = Vector2.ZERO
	effects.clear()
	dash.clear()
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
	elif model.phase == VICTORY and bridge.next_level():
		_reset_presentation()
	else:
		_restart()

func _refresh_model() -> void:
	model = JSON.parse_string(bridge.snapshot())
	var phase := int(model.phase)
	model.phase = phase
	wave_label.text = "沙盒" if phase == SANDBOX else ("第 %d/%d 波" % [model.wave, model.wave_count])
	for id in level_buttons:
		var button: Button = level_buttons[id]
		button.toggle_mode = true
		button.set_pressed_no_signal(model.level_id == id)
	companion_button.visible = phase == SANDBOX
	if phase == SANDBOX:
		companion_button.text = "黑狼出战: 开" if model.hellhound.sandbox_enabled else "黑狼出战: 关"
	if current_phase != phase:
		current_phase = phase
		next_button.visible = phase in [INTERMISSION, VICTORY, DEFEAT]
		match phase:
			INTERMISSION: next_button.text = "下一波"
			VICTORY: next_button.text = "下一关"
			_: next_button.text = "再来一局"
		match phase:
			INTERMISSION:
				hint.text = "拖动果苗或风铃换位"
			VICTORY:
				hint.text = "胜利 · 果园守住了！"
			DEFEAT:
				hint.text = "失败 · 果园失守"
			_:
				hint.text = "左键切果 · 右键 Home Run"
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
	_drain_events()

func _drain_events() -> void:
	var events: Array = JSON.parse_string(bridge.take_events())
	for i in events.size():
		var combo_hit := false
		if events[i][0] == "hit":
			# Core emits hit, optional die, then bounce and combo.
			var next := i + 1
			if next < events.size() and events[next][0] == "die":
				next += 1
			combo_hit = next + 1 < events.size() and events[next][0] == "bounce" \
				and events[next + 1][0] == "combo_bounces" and events[next + 1][3]
		_effect(events[i], combo_hit)

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton and event.button_index in [MOUSE_BUTTON_LEFT, MOUSE_BUTTON_RIGHT]:
		mouse = event.position
		if event.pressed:
			if event.button_index == MOUSE_BUTTON_RIGHT and not event.shift_pressed:
				var target := mouse - world.position
				if bridge.trigger_home_run(target.x, target.y):
					_refresh_model()
					_drain_events()
					return
			if _begin_drag(event.button_index, event.shift_pressed):
				return
			if event.button_index == MOUSE_BUTTON_LEFT and model.phase in [WAVE, SANDBOX]:
				swiping = true
				swipe_moved = false
				press_at = mouse
				trail.clear()
		else:
			if event.button_index == drag_button:
				_finish_drag()
			if swiping and event.button_index == MOUSE_BUTTON_LEFT:
				if not swipe_moved:
					var target := mouse - world.position
					bridge.click_cut(target.x, target.y)
					_refresh_model()
					_drain_events()
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
		bridge.slash_cut(from - world.position, event.position - world.position, vx if trail_angled else 0.0)
		_refresh_model()
		_drain_events()

func _slot_near(point: Vector2, distance: float) -> int:
	for i in slot_x.size():
		if (point - world.position).distance_to(_fruit_center(i)) <= distance:
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
	return index + slot_x.size() if index >= 0 and index < limit else -1

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
	var item = model.slots[index] if index < slot_x.size() else model.reserve[index - slot_x.size()]
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
			target = _tray_index(mouse, dragged_item < slot_x.size())
		if target >= 0:
			bridge.swap_plants(dragged_item, target)
	_stop_drag()
	_refresh_model()

func _stop_drag() -> void:
	drag_button = 0
	dragged_item = -1
	drag_item = {}

func _fruit_center(index: int) -> Vector2:
	return _vec(model.slot_geometry[index].fruit_center)

func _focused_fruit() -> int:
	var closest := -1
	var distance := INF
	for i in int(model.unlocked_slots):
		var radius = model.slot_geometry[i].cut_radius
		var offset := (mouse - world.position).distance_to(_fruit_center(i))
		if radius != null and offset <= radius and offset < distance:
			closest = i
			distance = offset
	return closest

func _paint_hud() -> void:
	hud.draw_rect(Rect2(0, 0, 480, 26), INK)
	hud.draw_rect(Rect2(0, 26, 480, 2), Color("#466179"))
	for i in 5:
		hud.draw_texture(hearts[0 if i < model.protagonist.hp else 1], Vector2(12 + 14 * i, 7))
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
			if i >= size or dragged_item == i + slot_x.size():
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
	_paint_protagonist()
	var active: bool = model.phase in [WAVE, SANDBOX]
	var focus := _focused_fruit() if active and dragged_item < 0 else -1
	if focus >= 0:
		var center := _fruit_center(focus)
		var x := roundf(center.x)
		var floor_y := roundi(_ground(x))
		for y in range(roundi(center.y) + 9, floor_y, 6):
			world.draw_rect(Rect2(x, y, 1, mini(3, floor_y - y)), Color("#ffffff88"))
		world.draw_rect(Rect2(x - 4, floor_y, 9, 1), Color("#fff8caaa"))
	for i in slot_x.size():
		var x: float = slot_x[i]
		var item = model.slots[i]
		if i >= model.unlocked_slots:
			world.draw_rect(Rect2(x - 10, 55, 21, 18), Color("#46586488"))
			for dx in [-8, -3, 2, 7]:
				world.draw_rect(Rect2(x + dx, 58, 2, 13), Color("#536373"))
			world.draw_rect(Rect2(x - 9, 62, 19, 2), Color("#677989"))
			continue
		if dragged_item >= 0:
			world.draw_rect(Rect2(_fruit_center(i).round() - Vector2(10, 8), Vector2(21, 20)), Color("#fff0a9c8"), false)
		if item == null:
			world.draw_rect(Rect2(x - 1, 59, 3, 3), Color("#446252"))
		elif dragged_item != i:
			var center := _fruit_center(i)
			if item.type == "fruit":
				var geometry: Dictionary = model.slot_geometry[i]
				var hook := _vec(geometry.hook)
				var angle := -float(geometry.sway_angle)
				world.draw_set_transform(hook, angle)
				world.draw_line(Vector2.ZERO, Vector2(0, hook.distance_to(center)), Color("#316b46"), 2)
				world.draw_rect(Rect2(-3, 6, 3, 2), Color("#93ba55"))
				world.draw_rect(Rect2(1, 11, 3, 2), Color("#6da650"))
				world.draw_set_transform(center.round(), angle)
				_stamp_item(item, Vector2.ZERO, i)
				world.draw_set_transform(Vector2.ZERO)
			else:
				_stamp_item(item, center, i)
	for barrier in model.barricades:
		_paint_barricade(barrier)
	for body in model.bodies:
		var pos := _vec(body.pos)
		var scaling := Vector2.ONE * float(body.scale)
		if body.kind == 1 and body.velocity[1] < -300:
			scaling *= Vector2(0.86, 1.14)
		var texture := green_fruits[int(body.kind)] if body.stage == 0 else fruit_textures[int(body.kind)]
		if body.home_run and body.velocity[0] > 200.0:
			var direction := _vec(body.velocity).normalized()
			for i in 3:
				var streak := pos + Vector2(0, -4 + i * 4)
				world.draw_line((streak - direction * (26 - i * 3)).round(),
					(streak - direction * 7).round(), Color("#fffffff0"), 1)
		if body.rolling:
			world.draw_set_transform(pos.round(), floorf(pos.x / 6.0) * PI / 8.0, scaling)
			world.draw_texture(texture, -texture.get_size() / 2.0)
			world.draw_set_transform(Vector2.ZERO)
		else:
			_stamp(texture, pos, scaling)
	for enemy in model.monsters:
		if enemy.kind == 3:
			continue # The wild enemy is drawn from the hellhound snapshot.
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
	if model.hellhound.present:
		_paint_hellhound(model.hellhound)
	if dragged_item >= 0:
		_stamp_item(drag_item, mouse - world.position, -1)
	if trail.size() > 1:
		var tint := SLASH if trail_angled else PAPER
		tint.a = clampf(trail_life * 15, 0.0, 1.0)
		for i in range(1, trail.size()):
			if trail[i - 1].distance_to(trail[i]) > 0.2:
				world.draw_line((trail[i - 1] - world.position).round(), (trail[i] - world.position).round(),
					tint, 1.0 if i < trail.size() - 2 else 2.0)
	for effect in effects:
		var age: float = effect.until - elapsed
		var pos: Vector2 = effect.pos
		if effect.has("hearts"):
			var time := 1.0 - age / float(effect.hearts)
			for i in 3:
				var rise := Vector2((i - 1) * (6 + 10 * time), -8 - 29 * time + (i % 2) * 6)
				_stamp(hearts[0], pos + rise, Vector2.ONE * 0.65, Color(1, 1, 1, clampf(age * 4, 0, 1)))
		elif effect.has("text"):
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

func _paint_hellhound(dog: Dictionary) -> void:
	var pos := _vec(dog.pos)
	var pose: String = dog.state
	if pose == "wild":
		pose = "patrol"
	elif pose == "patrol" and elapsed < hound_charge_until:
		pose = "charge"
	var direction := 1.0 if dog.facing_right else -1.0
	var frame := floori(elapsed * (22 if pose == "charge" else 9)) % 2
	var center := (pos + Vector2(0, 2)).round()
	if pose == "charge":
		center.x += direction * 3
		for i in 3:
			var streak := center + Vector2(-direction * (16 + i * 3), i * 3)
			world.draw_line(streak, streak - Vector2(direction * 7, 0), PAPER)
	world.draw_set_transform(center, 0.0, Vector2(direction, 1.0))
	_stamp(hound_frames[pose][frame], Vector2.ZERO)
	world.draw_set_transform(Vector2.ZERO)
	if dog.mode == "wild":
		var bar := Vector2(clampf(pos.x - 17, 1, 445), pos.y - 12).round()
		world.draw_rect(Rect2(bar, Vector2(34, 15)), INK)
		world.draw_string(FONT, bar + Vector2(7, 9), "驯服", HORIZONTAL_ALIGNMENT_LEFT, -1, 9, PAPER)
		world.draw_rect(Rect2(bar + Vector2(2, 11), Vector2(30, 2)), Color("#aa544c"))
		world.draw_rect(Rect2(bar + Vector2(2, 11),
			Vector2(roundf(30.0 * dog.tame_progress / dog.wild_max_hp), 2)), GOLD)
	elif pose == "tamed":
		var words := "小黑加入果园！"
		var width := FONT.get_string_size(words, HORIZONTAL_ALIGNMENT_LEFT, -1, 15).x
		var x := roundf((480 - width) / 2)
		world.draw_rect(Rect2(x - 12, 103, width + 24, 26), INK)
		world.draw_rect(Rect2(x - 12, 103, width + 24, 1), GOLD)
		world.draw_string(FONT, Vector2(x, 121), words, HORIZONTAL_ALIGNMENT_LEFT, -1, 15, PAPER)
		_stamp(hearts[0], pos + Vector2(0, -14 + sin(elapsed * 7) * 2), Vector2.ONE * 0.65)
	else:
		var pips := (pos + Vector2(-8, -9)).round()
		for i in int(dog.max_stamina):
			var at := pips + Vector2(i * 6, 0)
			world.draw_rect(Rect2(at, Vector2(5, 4)), INK)
			world.draw_rect(Rect2(at + Vector2.ONE, Vector2(3, 2)),
				GOLD if i < dog.stamina else Color("#5b6179"))
		if pose == "nap":
			_orbit_stars(pos - Vector2(0, 14), 10.0)
			world.draw_string(FONT, pos.round() + Vector2(-13, -22), "%.1fs" % dog.nap_seconds_left,
				HORIZONTAL_ALIGNMENT_LEFT, -1, 9, INK)

func _orbit_stars(center: Vector2, radius: float) -> void:
	for i in 3:
		var angle := elapsed * 10.0 + TAU * i / 3.0
		var at := (center + Vector2(cos(angle) * radius, sin(angle) * 3.0)).round()
		world.draw_rect(Rect2(at - Vector2(0, 2), Vector2(1, 5)), GOLD)
		world.draw_rect(Rect2(at - Vector2(2, 0), Vector2(5, 1)), GOLD)
		world.draw_rect(Rect2(at, Vector2.ONE), PAPER)

func _dash_progress(age: float) -> float:
	return clampf(minf(age / 0.035, (DASH_VFX_TIME - age) / 0.09), 0.0, 1.0)

func _paint_protagonist() -> void:
	var door := _vec(model.protagonist.pos)
	var hero := door
	var charging := float(model.protagonist.home_run_charge)
	var tool_angle := -0.78
	if not dash.is_empty():
		var age := elapsed - float(dash.started)
		if age < DASH_VFX_TIME:
			var start: Vector2 = dash.from
			var finish: Vector2 = dash.target + Vector2(-12, 4)
			for delay in [0.042, 0.027, 0.012]:
				var afterimage := start.lerp(finish, _dash_progress(age - delay))
				if afterimage.distance_to(start) > 3:
					_stamp(chief_flash, afterimage, Vector2.ONE, Color(0.85, 0.95, 1.0, 0.26))
			hero = start.lerp(finish, _dash_progress(age))
			tool_angle = lerpf(-1.85, 0.55, clampf((age - 0.035) / 0.075, 0.0, 1.0))
			if age >= 0.035 and age < 0.15:
				_paint_swing_fan(finish + Vector2(4, 0), 1.0 - (age - 0.035) / 0.115)
	var flashing := int(model.protagonist.hit_flash) % 8 >= 3
	_stamp(chief_flash if flashing else chief, hero)
	if elapsed < chief_bonk_until:
		_orbit_stars(hero - Vector2(0, 11), 11.0)
	var grip := hero + Vector2(4, 3)
	var tool := (grip + Vector2.from_angle(tool_angle) * 12).round()
	world.draw_line(grip.round(), tool, Color("#805042"), 2)
	world.draw_rect(Rect2(tool - Vector2(2, 1), Vector2(4, 3)), Color("#fff3ba") if charging >= 1.0 else GOLD)
	var bar := (door + Vector2(-9, 11)).round()
	world.draw_rect(Rect2(bar, Vector2(18, 4)), INK)
	world.draw_rect(Rect2(bar + Vector2.ONE, Vector2(floorf(charging * 16), 2)),
		Color("#faffed") if charging >= 1.0 else GOLD)
	if charging >= 1.0:
		var glint := tool + Vector2(3, -3)
		var color := Color(1.0, 1.0, 0.87, 0.7 + sin(elapsed * 7.0) * 0.25)
		world.draw_rect(Rect2(glint - Vector2(0, 2), Vector2(1, 5)), color)
		world.draw_rect(Rect2(glint - Vector2(2, 0), Vector2(5, 1)), color)
	if model.protagonist.digestion_cooldown > 0:
		var bubble := (door + Vector2(16, -22)).round()
		world.draw_rect(Rect2(bubble, Vector2(13, 9)), INK)
		world.draw_rect(Rect2(bubble + Vector2.ONE, Vector2(11, 7)), PAPER)
		world.draw_rect(Rect2(bubble + Vector2(2, 9), Vector2(2, 2)), PAPER)
		for i in 3:
			var offset := int(i == floori(elapsed * 6) % 3)
			world.draw_rect(Rect2(bubble + Vector2(3 + 3 * i, 4 - offset), Vector2(2, 2)), INK)

func _paint_swing_fan(at: Vector2, opacity: float) -> void:
	var vertices := PackedVector2Array([at.round()])
	for i in 7:
		vertices.append((at + Vector2.from_angle(lerpf(-1.4, 0.75, i / 6.0)) * 24).round())
	world.draw_colored_polygon(vertices, Color(1.0, 1.0, 1.0, opacity * 0.72))
	world.draw_polyline(vertices.slice(1), Color(1.0, 1.0, 1.0, opacity), 2.0)

func _effect(event: Array, combo_hit := false) -> void:
	var pos := _vec(event[1])
	match event[0]:
		"cut", "green_cut":
			var green_cut: bool = event[0] == "green_cut"
			_debris(pos, GREEN if green_cut else FRUIT_COLORS[int(event[2])], 5)
			var angled := not is_zero_approx(float(event[3]))
			effects.append({"pos": pos, "until": elapsed + 0.15,
				"slash": Vector2(float(event[3]) / 90.0, 1).normalized() * 18.0,
				"color": GREEN if green_cut else (SLASH if angled else PAPER)})
			audio.play(event[0])
		"shatter":
			_debris(pos, FRUIT_COLORS[int(event[2])], 10)
		"green_splat":
			_debris(pos, Color("#86cf37"), 12)
			_debris(pos, Color("#e0ff90"), 6)
		"hit":
			for enemy in model.monsters:
				if _vec(enemy.pos).distance_to(pos) < 6.0:
					flashes[enemy.id] = elapsed + 0.095
			if not combo_hit:
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
		"combo_bounces":
			audio.play_combo(int(event[2]))
			if event[3]:
				hitstop = maxf(hitstop, COMBO_HITSTOP)
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
			chief_bonk_until = elapsed + 0.65
			_debris(_vec(model.protagonist.pos) - Vector2(0, 6), Color("#c6f1ff"), 8)
			audio.play("bonk")
		"home_run":
			dash = {"from": _vec(event[2]), "target": pos, "started": elapsed}
			hitstop = maxf(hitstop, 0.05)
			shake = maxf(shake, 2.0)
			_debris(pos, Color.WHITE, 11)
			audio.play("home_run")
		"shield":
			effects.append({"pos": pos, "until": elapsed + 0.24, "color": Color("#fff1ba")})
			_debris(pos, GOLD, 10)
			audio.play("shield")
		"tame_progress":
			_debris(pos, GOLD, 8)
			audio.play("hit")
		"tamed", "hellhound_revived":
			var duration := 0.95 if event[0] == "tamed" else 0.65
			effects.append({"pos": pos, "until": elapsed + duration, "hearts": duration})
			_debris(pos - Vector2(0, 8), GOLD, 10)
			_debris(pos - Vector2(0, 8), PAPER, 6)
			audio.play("heal")
		"hellhound_charge":
			hound_charge_until = elapsed + 0.12
			_debris(pos + Vector2(0, 8), Color("#b78c68"), 6)
			audio.play("charge")
		"hellhound_hit":
			hound_charge_until = elapsed + 0.12
			effects.append({"pos": pos, "until": elapsed + 0.42, "text": str(int(event[2])),
				"giant": false, "color": GOLD})
			_debris(pos, GOLD, 7)
			audio.play("hit")
		"hellhound_nap":
			hound_charge_until = 0.0
			audio.play("bonk")

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
