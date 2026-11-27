extends Node2D

const Art = preload("res://scripts/art.gd")
const RetroAudio = preload("res://scripts/retro_audio.gd")
const FONT = preload("res://assets/SlopeOrchardSansSC.ttf")
const SLOTS := [110.0, 165.0, 220.0, 275.0, 330.0, 385.0]
const MOUNTS := [137.5, 192.5, 247.5, 302.5, 357.5]
const CLOUDS := [
	Vector3i(14, 70, 31), Vector3i(88, 35, 45), Vector3i(174, 79, 37),
	Vector3i(262, 31, 42), Vector3i(346, 90, 50), Vector3i(415, 54, 48),
]
const HOOK_Y := 62.0
const RESERVE := Vector2(360, 244)
const INK := Color("#202b3e")
const PAPER := Color("#fff4d9")
const GOLD := Color("#ffcd72")
const FRUIT_COLORS := [Color("#f4504f"), Color("#a2d45d"), Color("#ffad47")]
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
var fruit_flashes: Array[ImageTexture] = []
var monsters: Array[ImageTexture] = []
var monster_flashes: Array[ImageTexture] = []
var bell: ImageTexture
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
var swiping := false
var visited := {}
var dragged_plant := -1
var dragged_chime := -1
var drag_button := 0
var drag_kind := 0
var drag_scale := 1.0
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
		fruit_flashes.append(Art.sprite(Art.FRUIT[i], Art.FRUIT_PALETTES[i], true))
	var slime_colors := {"O": "#23483c", "L": "#68c35c", "D": "#388e52", "H": "#c6ed85", "W": "#ffffdc"}
	var boar_colors := {"O": "#302d39", "G": "#8497a0", "B": "#506778", "H": "#c9dade",
		"W": "#fff1c7", "P": "#a47159", "R": "#bb8567", "D": "#74504b"}
	monsters.assign([Art.sprite(Art.SLIME, slime_colors), Art.sprite(Art.BOAR, boar_colors)])
	monster_flashes.assign([Art.sprite(Art.SLIME, slime_colors, true), Art.sprite(Art.BOAR, boar_colors, true)])
	bell = Art.sprite(Art.BELL, {"O": "#624238", "G": "#ffd86c", "P": "#f2af53", "D": "#ca7e43", "H": "#fff1ac"})
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
				hint.text = "拖动果苗换位 · 催熟风铃放在两钩之间"
			VICTORY:
				hint.text = "胜利 · 果园守住了！"
			DEFEAT:
				hint.text = "失败 · 木屋失守"
			SANDBOX:
				hint.text = "左键切果 · 右键拖动果苗 · 拖动风铃"
			_:
				hint.text = "点按或按住左键划过果实"
	hud.queue_redraw()
	world.queue_redraw()

func _process(delta: float) -> void:
	elapsed += delta
	mouse = get_global_mouse_position()
	if not Input.is_mouse_button_pressed(MOUSE_BUTTON_LEFT):
		swiping = false
	if dragged_plant >= 0 or dragged_chime >= 0:
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
				visited.clear()
				trail.clear()
				_swipe(mouse, mouse)
		else:
			if event.button_index == drag_button:
				_finish_drag()
			if event.button_index == MOUSE_BUTTON_LEFT:
				swiping = false
	if event is InputEventMouseMotion and swiping:
		_swipe(event.position - event.relative, event.position)

func _slot_near(point: Vector2, distance: float) -> int:
	for i in SLOTS.size():
		if point.distance_to(Vector2(SLOTS[i], HOOK_Y)) <= distance:
			return i
	return -1

func _mount_near(point: Vector2, distance: float) -> int:
	for i in MOUNTS.size():
		if point.distance_to(Vector2(MOUNTS[i], HOOK_Y)) <= distance:
			return i
	return -1

func _begin_drag(button: int, shift: bool) -> bool:
	if model.phase not in [INTERMISSION, SANDBOX]:
		return false
	var mount := _mount_near(mouse, 12)
	if mount >= 0 and model.chimes[mount]:
		dragged_chime = mount
		drag_button = button
		return true
	if model.phase == SANDBOX and button != MOUSE_BUTTON_RIGHT and not shift:
		return false
	var slot := _slot_near(mouse, 19)
	if model.phase == INTERMISSION and mouse.distance_to(RESERVE) < 20.0:
		slot = 6
	if slot < 0:
		return false
	var fruit = model.reserve if slot == 6 else model.slots[slot]
	if fruit == null:
		return false
	dragged_plant = slot
	drag_button = button
	drag_kind = fruit.kind
	drag_scale = fruit.scale
	return true

func _finish_drag() -> void:
	if dragged_plant >= 0:
		var target := _slot_near(mouse, 24)
		if mouse.distance_to(RESERVE) < 23.0 and model.reserve != null:
			target = 6
		if target >= 0:
			bridge.swap_plants(dragged_plant, target)
	if dragged_chime >= 0:
		var target := _mount_near(mouse, 25)
		if target >= 0:
			bridge.move_chime(dragged_chime, target)
	_stop_drag()
	_refresh_model()

func _stop_drag() -> void:
	drag_button = 0
	dragged_plant = -1
	dragged_chime = -1

func _fruit_center(index: int) -> Vector2:
	return Vector2(SLOTS[index] + sin(elapsed * 2.6 + index * 1.2) * 1.5, HOOK_Y)

func _swipe(from: Vector2, to: Vector2) -> void:
	if trail.is_empty():
		trail.append(from)
	trail.append(to)
	if trail.size() > 8:
		trail.pop_front()
	trail_life = 0.09
	for i in SLOTS.size():
		var fruit = model.slots[i]
		if fruit == null or not fruit.hanging or visited.has(i):
			continue
		var center := _fruit_center(i) - Vector2(0, 2)
		var closest := Geometry2D.get_closest_point_to_segment(center, from, to) if from != to else to
		if center.distance_to(closest) <= maxf(6.0, 9.0 * fruit.scale):
			if bridge.cut_fruit(i):
				visited[i] = true
	_refresh_model()

func _paint_hud() -> void:
	hud.draw_rect(Rect2(0, 0, 480, 26), INK)
	hud.draw_rect(Rect2(0, 26, 480, 2), Color("#466179"))
	for i in 5:
		hud.draw_texture(hearts[0 if i < model.get("hp", 5) else 1], Vector2(81 + 14 * i, 7))
	match current_phase:
		INTERMISSION, VICTORY, DEFEAT:
			hud.draw_rect(Rect2(0, 227, 480, 43), INK)
			hud.draw_rect(Rect2(0, 227, 480, 2), GOLD)
			if model.reserve != null and dragged_plant != 6:
				hud.draw_string(FONT, Vector2(319, 249), "备用", HORIZONTAL_ALIGNMENT_LEFT, -1, 10, GOLD)
				hud.draw_texture_rect(fruit_textures[model.reserve.kind], Rect2(RESERVE - Vector2(8, 8), Vector2(16, 16)), false)
		_:
			hud.draw_rect(Rect2(0, 245, int(hint.get_minimum_size().x) + 24, 19), Color("#263146d9"))

func _stamp(texture: Texture2D, center: Vector2, scale: Vector2 = Vector2.ONE) -> void:
	var extent := (texture.get_size() * scale).round()
	world.draw_texture_rect(texture, Rect2((center - extent / 2.0).round(), extent), false)

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
	var active: bool = model.phase in [WAVE, SANDBOX]
	var focus := _slot_near(mouse, 14.0)
	if active and focus >= 0 and model.slots[focus] != null and model.slots[focus].hanging:
		var x: float = SLOTS[focus]
		var floor_y := roundi(155.0 + 0.194 * (x - 70.0))
		for y in range(77, floor_y, 6):
			world.draw_rect(Rect2(x, y, 1, mini(3, floor_y - y)), Color("#ffffff88"))
		world.draw_rect(Rect2(x - 4, floor_y, 9, 1), Color("#fff8caaa"))
	for i in SLOTS.size():
		var x: float = SLOTS[i]
		var fruit = model.slots[i]
		if dragged_plant >= 0:
			world.draw_rect(Rect2(x - 10, HOOK_Y - 9, 20, 20), Color("#fff0a9c8"), false)
		if fruit == null:
			world.draw_rect(Rect2(x - 1, 59, 3, 3), Color("#446252"))
		elif dragged_plant != i:
			var center := _fruit_center(i)
			world.draw_line(Vector2(x, 57), center.round(), Color("#438844"), 1)
			world.draw_rect(Rect2(x - 2, 58, 3, 2), Color("#93ba55"))
			if fruit.hanging:
				var flash: bool = fruit.stage == 2 and int(fruit.giant_age) % 104 < 5
				var texture := fruit_flashes[int(fruit.kind)] if flash else fruit_textures[int(fruit.kind)]
				_stamp(texture, center, Vector2.ONE * float(fruit.scale))
			else:
				world.draw_rect(Rect2(center.round() - Vector2(1, 2), Vector2(3, 3)), Color("#568e4f"))
	for i in MOUNTS.size():
		var center := Vector2(MOUNTS[i], HOOK_Y)
		if dragged_chime >= 0:
			world.draw_rect(Rect2(center.round() - Vector2(7, 7), Vector2(14, 15)), Color("#fff0a9c8"), false)
		if model.chimes[i] and dragged_chime != i:
			_stamp(bell, center + Vector2(round(sin(elapsed * 4.0 + i)), 0))
		elif not active:
			world.draw_rect(Rect2(center - Vector2(1, 4), Vector2(3, 2)), Color("#9b8157"))
	for body in model.bodies:
		var pos := _vec(body.pos)
		var scaling := Vector2.ONE * float(body.scale)
		if body.kind == 1 and body.velocity[1] < -300:
			scaling *= Vector2(0.86, 1.14)
		var texture := fruit_textures[int(body.kind)]
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
			var lift := 155.0 + 0.194 * (pos.x - 70.0) - 7.0 - pos.y
			squash = Vector2(1.08 - lift * 0.028, 0.87 + lift * 0.045)
		var texture := monster_flashes[int(enemy.kind)] if struck else monsters[int(enemy.kind)]
		_stamp(texture, pos + Vector2(0, 2 if enemy.kind == 0 else 3), squash)
		var width := 16 if enemy.kind == 1 else 12
		var bar := (pos + Vector2(-width / 2.0, -14 if enemy.kind == 1 else -10)).round()
		world.draw_rect(Rect2(bar, Vector2(width, 3)), INK)
		world.draw_rect(Rect2(bar + Vector2.ONE, Vector2(ceili((width - 2) * enemy.hp / enemy.max_hp), 1)), Color("#ed7958"))
	if dragged_plant >= 0:
		_stamp(fruit_textures[drag_kind], mouse, Vector2.ONE * drag_scale)
	if dragged_chime >= 0:
		_stamp(bell, mouse)
	if trail.size() > 1:
		var tint := Color(1, 1, 1, clampf(trail_life * 15, 0.0, 1.0))
		for i in range(1, trail.size()):
			if trail[i - 1].distance_to(trail[i]) > 0.2:
				world.draw_line(trail[i - 1].round(), trail[i].round(), tint, 1.0 if i < trail.size() - 2 else 2.0)
	for effect in effects:
		var age: float = effect.until - elapsed
		var pos: Vector2 = effect.pos
		if effect.has("text"):
			var at := (pos + Vector2(-3, -25 + age * 26)).round()
			var color: Color = GOLD if effect.giant else PAPER
			color.a = clampf(age * 5, 0.0, 1.0)
			world.draw_string_outline(FONT, at, effect.text, HORIZONTAL_ALIGNMENT_LEFT, -1, 13 if effect.giant else 10, 3, INK)
			world.draw_string(FONT, at, effect.text, HORIZONTAL_ALIGNMENT_LEFT, -1, 13 if effect.giant else 10, color)
			if effect.giant:
				world.draw_string(FONT, at + Vector2(1, 0), effect.text, HORIZONTAL_ALIGNMENT_LEFT, -1, 13, color)
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
			audio.play("cut")
		"shatter":
			_debris(pos, FRUIT_COLORS[int(event[2])], 10)
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
		"cabin":
			hitstop = maxf(hitstop, 0.04)
			shake = maxf(shake, 3)
			_debris(pos, Color("#ef795f"), 14)
			audio.play("hit")

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
