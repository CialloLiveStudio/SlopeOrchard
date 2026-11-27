extends SceneTree

const SCENE = preload("res://scenes/main.tscn")
const SLOTS := [110.0, 165.0, 220.0, 275.0, 330.0, 385.0]
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
	_check(game.model.phase == 0 and game.model.hp == 5, "Main scene did not initialize")
	_check(game.background.get_size() == Vector2(480, 270), "Pixel background has the wrong dimensions")
	_check_inputs(game)
	game._restart(false)
	var bridge: RefCounted = game.bridge
	_check(not bridge.cut_fruit(-1) and not bridge.cut_fruit(5), "Invalid cuts must be rejected")
	for wave in range(1, 5):
		var initial := _state(bridge)
		_check(initial.phase == 0 and initial.wave == wave, "Wrong campaign wave")
		if initial.phase != 0:
			break
		var events := _play_wave(bridge)
		var cleared := _state(bridge)
		print("Wave %d ended in phase %d with %d cabin hearts and %d hits." % [wave, cleared.phase, cleared.hp, events.get("hit", 0)])
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
				_check(cleared.slots[2].kind == 1 and cleared.chimes.count(true) == 1, "Missing first grant")
				_check(bridge.swap_plants(2, 5) and bridge.move_chime(2, 3), "First intermission drag actions failed")
			2:
				_check(_fruit_count(cleared) == 5, "Missing second grant")
			3:
				_check(_fruit_count(cleared) == 6 and cleared.reserve.kind == 2 and cleared.chimes.count(true) == 2, "Missing final grant")
				_check(bridge.swap_plants(6, 0) and bridge.swap_plants(0, 6), "Reserve swapping failed")
		_check(bridge.next_wave(), "Next wave action failed")

	bridge.reset_run()
	for i in 800:
		bridge.advance(0.1)
		bridge.drain_events()
		if _state(bridge).phase == 3:
			break
	var defeat := _state(bridge)
	_check(defeat.phase == 3 and defeat.hp == 0, "Idle run did not reach defeat")
	game._restart(true)
	_check(game.model.phase == 4 and _fruit_count(game.model) == 6 and game.model.chimes.count(true) == 2, "Sandbox loadout failed")
	_check(bridge.swap_plants(0, 1) and bridge.move_chime(0, 1), "Sandbox rearrangement failed")
	_check(bridge.swap_plants(0, 1) and bridge.move_chime(1, 0), "Sandbox restore failed")
	_check(bridge.cut_fruit(1), "Sandbox cut failed")
	var mixed := [false, false]
	var seen := {}
	for i in 350:
		bridge.advance(0.1)
		for monster in _state(bridge).monsters:
			mixed[int(monster.kind)] = true
		for event in JSON.parse_string(bridge.drain_events()):
			seen[event[0]] = true
			if i < 30:
				game._effect(event)
	_check(mixed.all(func(value): return value), "Sandbox did not spawn both enemies")
	_check(seen.has("bounce") and seen.has("ring") and seen.has("bowl"), "Sandbox chain reactions were missing")
	_check(_state(bridge).phase == 4, "Sandbox stopped")
	game._refresh_model()
	game._toggle_mute()
	_check(game.audio.muted, "Mute action failed")
	await process_frame
	game.queue_free()
	await process_frame
	if failures == 0:
		print("Slope Orchard probe passed: all four waves, victory, defeat, sandbox, scene, and bridge.")
	quit(0 if failures == 0 else 1)

func _fruit_count(state: Dictionary) -> int:
	return state.slots.filter(func(fruit): return fruit != null).size()

func _mouse_button(game: Node, at: Vector2, button: MouseButton, pressed: bool) -> void:
	var event := InputEventMouseButton.new()
	event.position = at
	event.button_index = button
	event.pressed = pressed
	game._unhandled_input(event)

func _check_inputs(game: Node) -> void:
	game._restart(true)
	_mouse_button(game, Vector2(110, 62), MOUSE_BUTTON_LEFT, true)
	_mouse_button(game, Vector2(110, 62), MOUSE_BUTTON_LEFT, false)
	_check(game.model.bodies.size() == 1 and not game.model.slots[0].hanging, "Mouse click failed to cut the fruit")
	_mouse_button(game, Vector2(150, 62), MOUSE_BUTTON_LEFT, true)
	var motion := InputEventMouseMotion.new()
	motion.position = Vector2(225, 62)
	motion.relative = Vector2(75, 0)
	game._unhandled_input(motion)
	_mouse_button(game, motion.position, MOUSE_BUTTON_LEFT, false)
	_check(game.model.bodies.size() == 3, "Swipe failed to cut two fruit in one motion")
	_mouse_button(game, Vector2(275, 62), MOUSE_BUTTON_RIGHT, true)
	_mouse_button(game, Vector2(330, 62), MOUSE_BUTTON_RIGHT, false)
	_check(game.model.slots[4].kind == 1, "Mouse drag failed to swap plants")
	_mouse_button(game, Vector2(137.5, 62), MOUSE_BUTTON_LEFT, true)
	_mouse_button(game, Vector2(192.5, 62), MOUSE_BUTTON_LEFT, false)
	_check(game.model.chimes[1] and not game.model.chimes[0], "Mouse drag failed to move the chime")

func _play_wave(bridge: RefCounted) -> Dictionary:
	var events := {}
	var recent := {}
	for frame in 3600:
		var state := _state(bridge)
		if state.phase != 0:
			break
		for i in SLOTS.size():
			var fruit = state.slots[i]
			if fruit == null or fruit.age < 40:
				continue
			var x: float = SLOTS[i]
			if fruit.kind == 2:
				if fruit.stage >= 1 and state.monsters.any(func(enemy): return enemy.pos[0] > x - 4):
					bridge.cut_fruit(i)
				continue
			for enemy in state.monsters:
				var speed := 29.0 if enemy.kind == 0 else 20.0
				var fall := sqrt(2.0 * maxf(0.0, enemy.pos[1] - 68.0 - fruit.scale * 7.0) / 610.0)
				var predicted_x: float = enemy.pos[0] - speed * fall
				var damage := 3 if fruit.stage == 2 else 2
				if fruit.kind == 0:
					damage = [1, 2, 4 if enemy.kind == 0 else 6][int(fruit.stage)]
				if absf(predicted_x - x) < 4.0 and frame >= recent.get(enemy.id, 0):
					if fruit.stage == 2 or damage >= enemy.hp or i <= 1 or fruit.kind == 1:
						if bridge.cut_fruit(i):
							recent[enemy.id] = frame + 32
							break
		bridge.advance(1.0 / 30.0)
		for event in JSON.parse_string(bridge.drain_events()):
			events[event[0]] = events.get(event[0], 0) + 1
	return events
