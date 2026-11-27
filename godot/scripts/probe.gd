extends SceneTree

const SCENE = preload("res://scenes/main.tscn")
const STEP := 1.0 / 120.0
var failures := 0

func _initialize() -> void:
	call_deferred("_run")

func _check(condition: bool, message: String) -> void:
	if not condition:
		failures += 1
		push_error(message)

func _state(bridge: RefCounted) -> Dictionary:
	return JSON.parse_string(bridge.snapshot())

func _run() -> void:
	var game := SCENE.instantiate()
	root.add_child(game)
	await process_frame
	game.set_process(false)
	game.audio.muted = true
	_check(game.model.phase == 0 and game.model.protagonist.hp == 5, "Main scene did not initialize")
	_check(game.background.get_size() == Vector2(480, 270), "Pixel background has the wrong dimensions")
	_check_inputs(game)
	for character in game.hint.text:
		_check(game.hint.get_theme_font("font").has_char(character.unicode_at(0)), "Control hint has a missing glyph")
	game.elapsed = game.dash.get("started", game.elapsed) + 0.07
	game.world.queue_redraw()
	await process_frame
	var bridge: RefCounted = game.bridge
	_check_campaign(bridge)
	bridge.reset_run()
	for i in 800:
		bridge.advance(0.1)
		bridge.drain_events()
		var state := _state(bridge)
		if state.phase == 3:
			break
		if state.phase == 1:
			for slot in int(state.unlocked_slots):
				if state.slots[slot] != null:
					_check(bridge.swap_plants(slot, state.slot_x.size() + _state(bridge).reserve.size()),
						"Could not empty the trellis for the defeat probe")
			_check(bridge.next_wave(), "Could not resume the undefended run")
	var defeat := _state(bridge)
	_check(defeat.phase == 3 and defeat.protagonist.hp == 0, "Undefended run did not reach defeat")
	_check(defeat.protagonist.hit_flash == 24 and defeat.protagonist.pos[0] < 62, "Protagonist hit snapshot missing")
	await _check_sandbox(game)
	_check_shield(bridge)
	_check_heal(game)
	game._toggle_mute()
	_check(not game.audio.muted, "Unmute action failed")
	game._toggle_mute()
	_check(game.audio.muted, "Mute action failed")
	game.queue_free()
	await process_frame
	if failures == 0:
		print("Slope Orchard v0.2.1 probe passed: campaign, defeat, sandbox, Flash-Step, digestion, input, and scene.")
	quit(0 if failures == 0 else 1)

func _check_campaign(bridge: RefCounted) -> void:
	bridge.reset_run()
	_check(not bridge.cut_fruit(-1) and not bridge.cut_fruit(5), "Invalid cuts must be rejected")
	var unlocks := [2, 3, 5, 6]
	var seen := {}
	for wave in range(1, 5):
		var initial := _state(bridge)
		_check(initial.phase == 0 and initial.wave == wave and initial.unlocked_slots == unlocks[wave - 1],
			"Incorrect campaign wave or slot unlocks")
		if initial.phase != 0:
			break
		var events := _play_wave(bridge, seen)
		var cleared := _state(bridge)
		print("Wave %d ended in phase %d with %d protagonist hearts, %d hits and %d Flash-Steps." %
			[wave, cleared.phase, cleared.protagonist.hp, events.get("hit", 0), events.get("home_run", 0)])
		_check(events.has("hit"), "No real collision in wave %d" % wave)
		_check(cleared.phase == (2 if wave == 4 else 1), "Wave %d did not clear" % wave)
		if cleared.phase != 1:
			break
		_check(not bridge.cut_fruit(0), "Intermission must pause cutting")
		var held: String = bridge.snapshot()
		bridge.advance(0.25)
		_check(held == bridge.snapshot(), "Intermission advanced the simulation")
		match wave:
			1:
				_check(cleared.slots[2].kind == 1 and cleared.unlocked_slots == 3, "Missing first grant")
				_check(not bridge.swap_plants(2, 5), "Locked slot accepted a placement")
			2:
				_check(cleared.unlocked_slots == 5 and cleared.slots[3].type == "chime"
					and cleared.slots[4].kind == 2, "Missing second grant")
				_check(bridge.swap_plants(2, 4), "Could not move the pumpkin uphill")
			3:
				_check(cleared.unlocked_slots == 6 and cleared.slots[5].kind == 2
					and cleared.reserve.size() == 1 and cleared.reserve[0].kind == 1, "Missing final grants")
				_check(bridge.swap_plants(4, 7) and _state(bridge).reserve.size() == 2, "Second tray cell failed")
				_check(bridge.swap_plants(7, 4), "Tray-to-empty slot transfer failed")
				_check(bridge.swap_plants(5, 3), "Could not place the second pumpkin uphill")
		_check(bridge.next_wave(), "Next wave action failed")
	_check(seen.get(2, 0) == 3, "Shield first appearance must be in wave 3")

func _mouse_button(game: Node, at: Vector2, button: MouseButton, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = at
	event.button_index = button
	event.pressed = pressed
	game._unhandled_input(event)

func _check_inputs(game: Node) -> void:
	game._restart(true)
	var slots: Array = game.model.slot_x
	var y: float = game.model.trellis_y
	var first := Vector2(slots[0], y)
	_check(first.x == 122 and slots[-1] < 480, "Shifted trellis layout failed")
	_mouse_button(game, first, MOUSE_BUTTON_LEFT, true)
	_mouse_button(game, first, MOUSE_BUTTON_LEFT, false)
	_check(game.model.bodies.size() == 1 and not game.model.slots[0].hanging
		and game.model.bodies[0].velocity[0] == 0.0, "Mouse click failed to drop vertically")
	var from := Vector2(slots[1], y - 2)
	_mouse_button(game, from, MOUSE_BUTTON_LEFT, true)
	var motion := InputEventMouseMotion.new()
	motion.position = Vector2(slots[2] + 5, y - 2)
	motion.relative = motion.position - from
	motion.velocity = Vector2(900, 0)
	game._unhandled_input(motion)
	_mouse_button(game, motion.position, MOUSE_BUTTON_LEFT, false)
	_check(game.model.bodies.size() == 3 and game.trail_angled, "Swipe failed to cut two stems")
	for body in game.model.bodies.slice(1):
		var angle := rad_to_deg(atan2(body.velocity[0], body.velocity[1]))
		_check(angle > 17.9 and angle <= 18.01, "Swipe velocity was not aimed and capped")
	var target := first + Vector2(0, 48)
	_mouse_button(game, target + Vector2.DOWN, MOUSE_BUTTON_RIGHT, true)
	_check(game.model.protagonist.home_run_cooldown == 0, "Flash-Step snapped outside 48 pixels")
	_mouse_button(game, target, MOUSE_BUTTON_RIGHT, true)
	_mouse_button(game, target, MOUSE_BUTTON_RIGHT, false)
	_check(game.model.protagonist.home_run_cooldown == 840 and game.model.protagonist.invulnerability_ticks > 0
		and game.model.bodies[0].home_run and game.hitstop == 0.05 and game.dash.target == first,
		"Right click failed to Flash-Step, set cooldown, or play dash VFX")
	_mouse_button(game, Vector2(slots[3], y), MOUSE_BUTTON_RIGHT, true)
	_mouse_button(game, Vector2(slots[4], y), MOUSE_BUTTON_RIGHT, false)
	_check(game.model.slots[4].type == "chime", "Mouse drag failed to move the chime")
	_mouse_button(game, Vector2(slots[4], y), MOUSE_BUTTON_RIGHT, true)
	_mouse_button(game, Vector2(244, 245), MOUSE_BUTTON_RIGHT, false)
	_check(game.model.reserve.size() == 2 and game.model.reserve[1].type == "chime", "Mouse drag failed to store chime")

func _check_sandbox(game: Node) -> void:
	game._restart(true)
	var bridge: RefCounted = game.bridge
	var state := _state(bridge)
	_check(state.phase == 4 and state.unlocked_slots == 6 and state.slots[3].type == "chime",
		"Sandbox loadout failed")
	_check(state.monsters.map(func(enemy): return int(enemy.kind)) == [0, 1, 2]
		and state.monsters[2].hp == 6, "Sandbox must start with all three monsters")
	_check(bridge.swap_plants(1, 2) and bridge.cut_fruit(2) and bridge.cut_fruit(0) and bridge.cut_fruit(5),
		"Sandbox opening cuts failed")
	var seen := {}
	var checked_barricade := false
	for tick in 2000:
		bridge.advance(STEP)
		state = _state(bridge)
		if state.protagonist.home_run_cooldown > 0:
			seen["home_run_cooldown"] = true
		if state.slots[3].cooldown > 0:
			_check(state.slots[3].cooldown <= 120, "Chime cooldown exceeds one second")
			seen["cooldown"] = true
		if state.bodies.any(func(body): return body.home_run and body.velocity[0] > 300):
			seen["projectile"] = true
		for barrier in state.barricades:
			if not checked_barricade:
				_check(barrier.body.kind == 2 and not barrier.body.rolling and int(barrier.hits_left) in [1, 2, 3],
					"Invalid parked pumpkin snapshot")
				checked_barricade = true
		if tick > 0 and tick % 90 == 0:
			_sandbox_cuts(bridge, state)
		if not seen.has("home_run") and tick >= 58:
			for body in state.bodies:
				if body.kind == 1 and bridge.trigger_home_run(body.pos[0], body.pos[1]):
					break
		for event in JSON.parse_string(bridge.drain_events()):
			if not seen.has(event[0]):
				game.model = state
				game._effect(event)
			seen[event[0]] = true
		if tick % 600 == 0:
			game._refresh_model()
			await process_frame
	print("Sandbox interactions: %s. Barricade %s." % [str(seen.keys()), checked_barricade])
	for key in ["bounce", "ring", "cooldown", "bowl", "home_run", "home_run_cooldown", "projectile"]:
		_check(seen.has(key), "Sandbox interaction missing: " + key)
	_check(checked_barricade, "Sandbox never exposed a parked pumpkin")
	_check(state.phase == 4, "Sandbox stopped")
	game._refresh_model()
	await process_frame

func _check_shield(bridge: RefCounted) -> void:
	bridge.start_sandbox()
	for i in 110:
		bridge.advance(STEP)
	_check(bridge.cut_fruit(5), "Could not drop a pumpkin over the approaching shield")
	for i in 85:
		bridge.advance(STEP)
	var events: Array = JSON.parse_string(bridge.drain_events())
	_check(events.any(func(event): return event[0] == "shield"), "Shield cap did not rebound the pumpkin")

func _check_heal(game: Node) -> void:
	game._restart(true)
	var bridge: RefCounted = game.bridge
	_check(bridge.swap_plants(0, 7), "Could not hold a giant apple in reserve")
	for i in 160:
		bridge.advance(0.1)
		if _state(bridge).protagonist.hp == 4:
			break
	_check(_state(bridge).protagonist.hp == 4, "Sandbox protagonist did not take damage")
	bridge.drain_events()
	_check(bridge.swap_plants(7, 0) and bridge.cut_fruit_angled(0, -29.2), "Could not swipe the reserve apple left")
	for i in 64:
		bridge.advance(STEP)
	var healed := _state(bridge)
	var events: Array = JSON.parse_string(bridge.drain_events())
	_check(healed.protagonist.hp == 5 and healed.protagonist.digestion_cooldown > 1700
		and events.any(func(event): return event[0] == "heal"),
		"Protagonist did not catch the apple and heal")
	game._refresh_model()
	for event in events:
		game._effect(event)

func _sandbox_cuts(bridge: RefCounted, state: Dictionary) -> void:
	for slot in [0, 2, 4, 5]:
		var fruit: Dictionary = state.slots[slot]
		if fruit.stage != 2 and not (slot == 5 and fruit.stage == 1):
			continue
		var x: float = state.slot_x[slot]
		if state.barricades.any(func(barrier): return absf(barrier.body.pos[0] - x) < 22):
			continue
		if slot in [0, 5] or state.monsters.any(func(enemy): return absf(enemy.pos[0] - x - 18.0) < 40):
			bridge.cut_fruit(slot)

func _play_wave(bridge: RefCounted, seen: Dictionary) -> Dictionary:
	var events := {}
	var recent := {}
	for frame in 3600:
		var state := _state(bridge)
		if state.phase != 0:
			break
		for enemy in state.monsters:
			if not seen.has(int(enemy.kind)):
				seen[int(enemy.kind)] = state.wave
		for i in int(state.unlocked_slots):
			var fruit = state.slots[i]
			if fruit == null or fruit.type != "fruit" or not fruit.hanging or fruit.stage == 0:
				continue
			var x: float = state.slot_x[i]
			if i == 0 and fruit.stage == 2 and state.protagonist.hp < 5 and state.protagonist.digestion_cooldown == 0:
				bridge.cut_fruit_angled(i, -29.2)
				continue
			if fruit.kind == 2:
				if state.monsters.any(func(enemy): return enemy.pos[0] > x + 3):
					bridge.cut_fruit(i)
				continue
			for enemy in state.monsters:
				var speed: float = [29.0, 20.0, 24.0][int(enemy.kind)]
				var fall := sqrt(2.0 * maxf(0.0, enemy.pos[1] - 68.0 - fruit.scale * 7.0) / 610.0)
				var predicted_x: float = enemy.pos[0] - speed * fall
				var damage: int = [1, 2, 6 if enemy.kind == 1 else 4][int(fruit.stage)] if fruit.kind == 0 else int(fruit.stage)
				if frame < recent.get(enemy.id, 0):
					continue
				if enemy.kind == 2:
					if absf(predicted_x - x - 17.0) < 2.5 and bridge.cut_fruit_angled(i, 29.2):
						recent[enemy.id] = frame + 22
						break
				elif absf(predicted_x - x) < 3.5 and (damage >= enemy.hp or i <= 1 or fruit.kind == 1):
					if bridge.cut_fruit(i):
						recent[enemy.id] = frame + 24
						break
		if state.protagonist.home_run_cooldown == 0:
			for body in state.bodies:
				if body.rolling or body.stage == 0 or body.home_run:
					continue
				var distance_to_lane: float = maxf(0, 155 + 0.194 * (body.pos[0] - 70) - body.pos[1] - 12)
				var ahead: float = distance_to_lane * 440.0 / 310.0
				if state.monsters.any(func(enemy): return enemy.pos[0] > body.pos[0] + ahead + 16):
					if bridge.trigger_home_run(body.pos[0], body.pos[1]):
						break
		bridge.advance(1.0 / 30.0)
		for event in JSON.parse_string(bridge.drain_events()):
			events[event[0]] = events.get(event[0], 0) + 1
	return events
