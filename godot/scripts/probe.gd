extends SceneTree

const SCENE = preload("res://scenes/main.tscn")
const STEP := 1.0 / 120.0
var failures := 0

func _initialize() -> void:
	# Fail if a runtime error interrupts the probe before it can quit.
	create_timer(30.0).timeout.connect(func(): quit(1))
	call_deferred("_run")

func _check(condition: bool, message: String) -> void:
	if not condition:
		failures += 1
		push_error(message)

func _state(bridge: RefCounted) -> Dictionary:
	return JSON.parse_string(bridge.snapshot())

func _events(bridge: RefCounted) -> Array:
	return JSON.parse_string(bridge.take_events())

func _has_event(events: Array, key: String) -> bool:
	return events.any(func(event): return event[0] == key)

func _mouse_button(game: Node, at: Vector2, button: MouseButton, pressed: bool, shift := false) -> void:
	var event := InputEventMouseButton.new()
	event.position = at
	event.button_index = button
	event.pressed = pressed
	event.shift_pressed = shift
	game._unhandled_input(event)

func _drag(game: Node, from: Vector2, to: Vector2, button: MouseButton, shift := false) -> void:
	_mouse_button(game, from, button, true, shift)
	_mouse_button(game, to, button, false, shift)

func _check_open_slots(game: Node, kinds: Array) -> void:
	_check(game.model.unlocked_slots == 6 and game.slot_x.size() == 6, "All six slots must start open")
	var actual: Array = game.model.slots.map(func(item): return null if item == null else int(item.kind))
	_check(actual == kinds, "Starting plants must occupy slots 0, 2, 4 and 5")
	_drag(game, game._fruit_center(0), game._fruit_center(1), MOUSE_BUTTON_LEFT, true)
	_drag(game, game._fruit_center(1), game._fruit_center(3), MOUSE_BUTTON_LEFT, true)
	_check(game.model.slots[0] == null and game.model.slots[1] == null and game.model.slots[3] != null,
		"Wave drag failed to use both empty slots")
	_drag(game, game._fruit_center(3), game._fruit_center(0), MOUSE_BUTTON_LEFT, true)

func _run() -> void:
	var game := SCENE.instantiate()
	root.add_child(game)
	await process_frame
	game.set_process(false)
	game.audio.muted = true
	_check(game.background.get_size() == Vector2(480, 270), "Pixel background has the wrong dimensions")
	if DisplayServer.get_name() != "headless":
		await RenderingServer.frame_post_draw
		var frame := root.get_texture().get_image()
		_check(_scene_pixel(frame, Vector2i(4, 4)).is_equal_approx(Color("#202b3e")),
			"Scene did not render the HUD")
		for slot in [1, 3]:
			var x := int(game.slot_x[slot])
			_check(_scene_pixel(frame, Vector2i(x, 60)).is_equal_approx(Color("#446252")),
				"Empty slot marker did not render")
	_check(game.model.level_id == "t1" and game.model.phase == 0, "Scene must start in T1")
	if game.model.get("level_id") != "t1":
		quit(1)
		return
	for button in game.level_buttons.values():
		for character in button.text:
			_check(game.FONT.has_char(character.unicode_at(0)), "Level button has a missing glyph")
	await _check_green_input(game)
	await _check_t1(game)
	await _check_t2(game)
	await _check_shield(game)
	await _check_sandbox(game)
	game._toggle_mute()
	_check(not game.audio.muted, "Unmute action failed")
	game._toggle_mute()
	_check(game.audio.muted, "Mute action failed")
	game.queue_free()
	await process_frame
	if failures == 0:
		print("Slope Orchard v0.3.1 probe passed: six-slot drag, vertical drops, wild reactions, shield lids, companions, and scene.")
	quit(0 if failures == 0 else 1)

func _scene_pixel(frame: Image, at: Vector2i) -> Color:
	return frame.get_pixelv(at * frame.get_size() / Vector2i(480, 270))

func _check_green_input(game: Node) -> void:
	var bridge: RefCounted = game.bridge
	_check(not bridge.start_level("unknown"), "Unknown level accepted")
	_check(not bridge.cut_fruit(-1) and not bridge.cut_fruit(1), "Invalid cuts accepted")
	_check(bridge.swap_plants(2, 1), "Could not set up neighboring vines")
	bridge.advance(0.25)
	bridge.advance(0.25)
	bridge.advance(11 * STEP)
	_check(not bridge.cut_fruit(0), "Fruit was cut before the green threshold")
	bridge.advance(STEP)
	game._refresh_model()
	var first: Vector2 = game._fruit_center(0)
	var slot: Dictionary = game.model.slots[0]
	_check(slot.stage == 0 and slot.hanging and slot.growth >= 0.15, "T1 apple did not hang at green threshold")
	_check(game.model.slot_geometry[0].cut_radius >= 22.0, "Green hitbox is smaller than 22 px")
	var outside := first + Vector2(0, 22.5)
	_check(not bridge.click_cut(outside.x, outside.y), "Green cut accepted outside its radius")
	var at := first + Vector2(0, 21)
	_drag(game, at, at, MOUSE_BUTTON_LEFT)
	_check(game.model.bodies.size() == 1 and not game.model.slots[0].hanging, "Green mouse click at 21 px missed")
	if game.model.bodies.size() != 1:
		return
	_check(game._vec(game.model.bodies[0].pos).is_equal_approx(game._drop_guide(0)),
		"Click drop did not begin on the guide")
	_check(game.effects.size() == 1, "Green click did not dispatch cut effects")
	_check(absf(game.model.slot_geometry[1].sway_angular_vel) > 0.01, "Green cut did not impart a neighbor impulse")
	bridge.advance(0.25)
	game._refresh_model()
	var geometry: Dictionary = game.model.slot_geometry[1]
	var center: Vector2 = game._fruit_center(1)
	var hook: Vector2 = game._vec(geometry.hook)
	var angle: float = geometry.sway_angle
	_check(absf(rad_to_deg(angle)) >= 8.0 and absf(rad_to_deg(angle)) <= 18.01, "Neighbor cut did not visibly swing the vine")
	_check(center.distance_to(hook + Vector2.DOWN.rotated(-angle) * 18.0) < 0.005,
		"Sway geometry disagrees with core pendulum")
	var guide: Vector2 = game._drop_guide(1)
	_check(is_equal_approx(guide.x, game.model.slot_x[1]) and is_equal_approx(guide.y, game.model.trellis_y)
		and absf(center.x - guide.x) > 2.0, "Guide followed the visual sway")
	_check(is_equal_approx(game.model.bodies[0].pos[0], game.model.slot_x[0])
		and is_zero_approx(game.model.bodies[0].velocity[0]), "Click drop drifted horizontally")
	game.mouse = center + Vector2(0, 21)
	_check(game._focused_fruit() == 1, "Drop guide did not focus the swayed green fruit")
	game.world.queue_redraw()
	await process_frame
	if DisplayServer.get_name() != "headless":
		await RenderingServer.frame_post_draw
		var frame := root.get_texture().get_image()
		var y := roundi(guide.y) + 42
		_check(_scene_pixel(frame, Vector2i(roundi(guide.x), y)).r > _scene_pixel(frame, Vector2i(roundi(center.x), y)).r,
			"Rendered drop guide did not stay on the launch column")
	var from := center + Vector2(-24, 21)
	_mouse_button(game, from, MOUSE_BUTTON_LEFT, true)
	var motion := InputEventMouseMotion.new()
	motion.position = center + Vector2(24, 21)
	motion.relative = motion.position - from
	motion.velocity = Vector2(900, 0)
	game._unhandled_input(motion)
	_mouse_button(game, motion.position, MOUSE_BUTTON_LEFT, false)
	_check(game.model.bodies.size() == 2 and game.model.bodies.back().stage == 0 and game.trail_angled
		and game._vec(game.model.bodies.back().pos).is_equal_approx(guide)
		and game.model.bodies.back().velocity[0] > 0.0,
		"Green slash at 21 px missed")
	for i in 4:
		bridge.advance(0.25)
	var events := _events(bridge)
	_check(_has_event(events, "green_splat"), "Green apple did not emit green_splat")
	for event in events:
		game._effect(event)
	await process_frame
	game._restart("t1")
	for i in 3:
		bridge.advance(0.25)
	_check(bridge.cut_fruit_angled(0, -29.2), "Could not slash a green apple toward the protagonist")
	for i in 3:
		bridge.advance(0.25)
	game._refresh_model()
	game._drain_events()
	_check(game.chief_bonk_until > game.elapsed and game.model.protagonist.hp == 5,
		"Green bonk did not show stars or altered the protagonist's health")
	await process_frame

func _check_t1(game: Node) -> void:
	game.level_buttons.t1.pressed.emit()
	var bridge: RefCounted = game.bridge
	_check(game.model.level_id == "t1" and game.model.wave_count == 3 and not game.model.hellhound.present,
		"T1 start snapshot is incorrect")
	_check_open_slots(game, [0, null, 0, null, 0, 0])
	_check(not bridge.set_sandbox_companion(true), "Companion toggle was accepted in T1")
	var wild_seen := false
	var reactions := {}
	var green_cut := false
	var home_run_cut := false
	var home_run := false
	var progress := 0
	for frame in 4000:
		var state := _state(bridge)
		if state.phase == 1:
			game._refresh_model()
			var first: Vector2 = game._fruit_center(0)
			var second: Vector2 = game._fruit_center(2)
			_drag(game, second, game._fruit_center(3), MOUSE_BUTTON_LEFT)
			_check(game.model.slots[2] == null and game.model.slots[3] != null,
				"Intermission drag could not use an empty slot")
			_drag(game, game._fruit_center(3), second, MOUSE_BUTTON_LEFT)
			var tray: Vector2 = game.RESERVE
			_drag(game, first, tray, MOUSE_BUTTON_LEFT)
			_drag(game, second, tray + Vector2(game.RESERVE_STEP, 0), MOUSE_BUTTON_LEFT)
			_check(game.model.reserve.size() == 2 and game.model.reserve.all(func(item): return item.kind == 0),
				"Intermission tray rejected duplicate apples")
			_drag(game, tray + Vector2(game.RESERVE_STEP, 0), first, MOUSE_BUTTON_LEFT)
			_drag(game, tray, second, MOUSE_BUTTON_LEFT)
			_check(game.model.reserve.is_empty() and game.model.slots[0] != null and game.model.slots[2] != null,
				"Intermission did not restore both apples")
			game.next_button.pressed.emit()
			continue
		if state.phase != 0:
			break
		if state.hellhound.present and not wild_seen:
			wild_seen = true
			_check(state.wave == 3 and state.hellhound.mode == "wild" and state.hellhound.state == "wild"
				and state.hellhound.tame_max == 100 and state.hellhound.tame_progress == 0,
				"Wild hellhound snapshot is incorrect")
			game._refresh_model()
			_check(game._tame_fraction(game.model.hellhound) == 0.0, "Tame bar did not start empty")
			await process_frame
		if progress < 75:
			_t1_cuts(bridge, state)
		elif progress == 75 and not green_cut:
			# The mature hit holds the hound in range while the next apple is green.
			var apple: Dictionary = state.slots[5]
			_check(apple.stage == 0 and apple.hanging, "Green apple was unavailable after the mature hit")
			green_cut = bridge.cut_fruit(5)
		elif progress == 78 and not home_run_cut:
			# Let a ripe apple fall from the next slot uphill before batting it right.
			if not state.slots[4].hanging or state.slots[4].stage == 0:
				for slot in [2, 0, 5]:
					var ripe: Dictionary = state.slots[slot]
					if ripe.stage > 0:
						bridge.swap_plants(slot, 4)
						break
			home_run_cut = bridge.cut_fruit(4)
		elif home_run_cut and not home_run:
			for body in state.bodies:
				if body.pos[0] == state.slot_x[4] and body.pos[1] >= 158.0 and body.stage > 0:
					home_run = bridge.trigger_home_run(body.pos[0], body.pos[1])
					break
		bridge.advance(4 * STEP)
		var events := _events(bridge)
		if _has_event(events, "tame_progress") or (_has_event(events, "wild_pant") and not reactions.has("pant")):
			game._refresh_model()
			for event in events:
				game._effect(event)
				match event[0]:
					"wild_pant":
						reactions.pant = game.wild_reaction.get("pose") == "pant"
					"wild_munch":
						var giant: bool = event[2] == 2
						var heart: Dictionary = game.effects.back()
						reactions["giant" if giant else "mature"] = (
							game.wild_reaction.get("pose") == "munch" and heart.get("heart_texture") == game.pink_heart
							and (heart.heart_scale > 1.0 if giant else heart.heart_scale < 1.0))
					"wild_sour":
						reactions.sour = game.wild_reaction.get("pose") == "sour" and game.effects.back().has("sweat")
					"wild_dizzy":
						reactions.dizzy = home_run and game.wild_reaction.get("pose") == "dizzy"
					"tame_progress":
						var next_progress := int(game.model.hellhound.tame_progress)
						_check(next_progress > progress and next_progress == int(event[3] - event[2])
							and is_equal_approx(game._tame_fraction(game.model.hellhound), next_progress / 100.0),
							"Tame bar did not advance with the snapshot")
						progress = next_progress
					"tamed":
						reactions.tamed = (game.wild_reaction.is_empty() and game.effects.back().has("hearts")
							and not game.effects.back().has("heart_texture"))
			await process_frame
	game._refresh_model()
	for reaction in ["pant", "mature", "giant", "sour", "dizzy", "tamed"]:
		_check(reactions.get(reaction, false), "T1 did not exercise the %s reaction" % reaction)
	_check(game.model.phase == 2 and game.model.hellhound.get("state") == "tamed"
		and game._tame_fraction(game.model.hellhound) == 1.0
		and game.model.companion_unlocked, "T1 did not reach the tamed celebration")
	await process_frame
	game.next_button.pressed.emit()
	_check(game.model.level_id == "t2", "T1 victory did not continue to T2")

func _t1_cuts(bridge: RefCounted, state: Dictionary) -> void:
	for i in state.slots.size():
		var fruit = state.slots[i]
		if fruit == null or fruit.stage == 0:
			continue
		for enemy in state.monsters:
			var speed := 16.0 if enemy.kind == 3 else 29.0
			var fall := sqrt(2.0 * maxf(0.0, enemy.pos[1] - state.trellis_y - 6.0) / 610.0)
			var relative_x: float = enemy.pos[0] - state.slot_x[i] - speed * fall
			if absf(relative_x) <= 4.5:
				bridge.cut_fruit(i)
				break

func _check_t2(game: Node) -> void:
	game.level_buttons.t2.pressed.emit()
	var bridge: RefCounted = game.bridge
	var dog: Dictionary = game.model.hellhound
	_check(game.model.level_id == "t2" and dog.present
		and dog.mode == "companion" and dog.state == "patrol" and dog.stamina == 3
		and dog.max_stamina == 3 and is_equal_approx(dog.nap_seconds_total, 12.0),
		"T2 companion or starting loadout missing")
	_check_open_slots(game, [0, null, 1, null, 2, 0])
	_check(bridge.equip_plant(0, 1), "T2 rejected a second melon")
	game._refresh_model()
	_check(game.model.slots[0].kind == 1 and game.model.slots[2].kind == 1, "Duplicate equip replaced an existing melon")
	bridge.advance(0.25)
	game._refresh_model()
	_check(game.model.hellhound.pos[0] > dog.pos[0], "Companion did not patrol")
	await process_frame
	game._restart()
	_check(game.model.level_id == "t2" and game.model.slots[0].kind == 0, "Reset did not restart the current level")

func _shield(state: Dictionary, id: int) -> Dictionary:
	for enemy in state.monsters:
		if int(enemy.id) == id:
			return enemy
	return {}

func _check_shield(game: Node) -> void:
	var bridge: RefCounted = game.bridge
	for mode in ["dent", "giant", "home_run"]:
		game._restart("sandbox")
		game.companion_button.pressed.emit()
		var shield: Dictionary = game.model.monsters[2]
		var id := int(shield.id)
		_check(shield.kind == 2 and shield.lid_hp == 3, "Shield did not spawn with a full lid")
		var previous_lid: Texture2D = game._shield_lid(3)
		for slot in game.slot_x.size():
			_check(bridge.swap_plants(slot, 7 + slot), "Could not park sandbox equipment for the shield probe")
		var shots := []
		match mode:
			"dent":
				# Let the same shield walk under three successive mature apples.
				shots = [[4, 0, 378], [3, 390, 643], [2, 650, 908]]
			"giant":
				_check(bridge.swap_plants(11, 5), "Could not place the giant apple")
				shots = [[5, -1, 113]]
			"home_run":
				shots = [[4, 0, 277]]
		var dents := []
		var broke := false
		var home_run := false
		for tick in 1100:
			for shot in shots:
				if tick == shot[1]:
					_check(bridge.equip_plant(shot[0], 0), "Could not grow a shield test apple")
				if tick == shot[2]:
					_check(bridge.cut_fruit(shot[0]), "Could not drop a shield test fruit")
			if mode == "home_run" and tick == 347:
				var bodies: Array = _state(bridge).bodies
				_check(bodies.size() == 1 and bodies[0].stage == 1, "Home Run test needs a mature apple")
				if bodies.size() == 1:
					var target: Vector2 = game._vec(bodies[0].pos)
					home_run = bridge.trigger_home_run(target.x, target.y)
			bridge.advance(STEP)
			var events := _events(bridge)
			if not (_has_event(events, "shield_lid_dent") or _has_event(events, "shield_lid_break")):
				continue
			game._refresh_model()
			shield = _shield(game.model, id)
			for event in events:
				var sound := String(event[0])
				if sound in ["shield_lid_dent", "shield_lid_break"]:
					var player: AudioStreamPlayer = game.audio.players[game.audio.voice]
					game.audio.muted = false
					game._effect(event)
					_check(player.stream == game.audio.sounds[sound], "Lid event played the wrong sound")
					game.audio.muted = true
				else:
					game._effect(event)
				if event[0] == "shield_lid_dent":
					var hp := int(event[2])
					dents.append(hp)
					var lid: Texture2D = game._shield_lid(hp)
					_check(shield.get("lid_hp") == hp and shield.get("hp") == 6 and lid != previous_lid,
						"Shield dent did not update the lid independently of body HP")
					previous_lid = lid
			await process_frame
			if _has_event(events, "shield_lid_break"):
				broke = true
				_check(shield.get("lid_hp") == 0 and game._shield_lid(0) == null,
					"Broken shield still has a lid")
				_check(game.effects.any(func(effect): return effect.has("lid")),
					"Shield break did not launch a lid particle")
				break
		_check(broke, "Shield did not break in the %s test (dents %s)" % [mode, str(dents)])
		_check(dents == ([2, 1, 0] if mode == "dent" else []), "Unexpected dent sequence for " + mode)
		_check(mode != "home_run" or home_run, "Shield test could not trigger Home Run")
	_check(game.audio.sounds.shield_lid_dent != game.audio.sounds.shield_lid_break,
		"Shield dent and break must have distinct sounds")

func _check_sandbox(game: Node) -> void:
	game.level_buttons.sandbox.pressed.emit()
	var bridge: RefCounted = game.bridge
	_check(game.model.level_id == "sandbox" and game.model.phase == 4 and game.model.unlocked_slots == 6
		and game.model.monsters.size() == 3 and game.model.slots[3].type == "chime",
		"Sandbox starting snapshot is incorrect")
	_check(game.companion_button.visible and game.model.hellhound.sandbox_enabled, "Sandbox toggle missing")
	game.companion_button.pressed.emit()
	_check(not game.model.hellhound.present and not game.model.hellhound.sandbox_enabled, "Sandbox toggle did not remove companion")
	game.companion_button.pressed.emit()
	_check(game.model.hellhound.present and game.model.hellhound.stamina == 3, "Sandbox toggle did not restore companion")
	var tray: Vector2 = game.RESERVE
	_drag(game, game._fruit_center(4), tray + Vector2(game.RESERVE_STEP, 0), MOUSE_BUTTON_RIGHT)
	_drag(game, game._fruit_center(5), tray + Vector2(game.RESERVE_STEP * 2, 0), MOUSE_BUTTON_RIGHT)
	_check(game.model.reserve.size() == 3 and game.model.reserve[1].kind == 0 and game.model.reserve[2].kind == 2,
		"Sandbox drag could not store fruit in new tray cells")
	_drag(game, tray + Vector2(game.RESERVE_STEP, 0), game._fruit_center(4), MOUSE_BUTTON_RIGHT)
	_check(game.model.slots[0].kind == 0 and game.model.slots[4].kind == 0,
		"Sandbox drag could not place a second apple")
	_check(bridge.swap_plants(4, 8), "Could not hold a giant apple in the tray")
	_check(bridge.cut_fruit(1), "Could not drop sandbox melon")
	game._refresh_model()
	if game.model.bodies.is_empty():
		return
	var target: Vector2 = game._vec(game.model.bodies[0].pos)
	_mouse_button(game, target, MOUSE_BUTTON_RIGHT, true)
	_check(game.model.bodies[0].home_run and is_equal_approx(game.hitstop, 0.05), "Flash-Step input or hitstop failed")
	var seen := {}
	for i in 750:
		bridge.advance(6 * STEP)
		var events := _events(bridge)
		for event in events:
			if not seen.has(event[0]):
				game._refresh_model()
				game._effect(event)
				await process_frame
			seen[event[0]] = true
		if seen.has("hellhound_nap"):
			break
	for key in ["bounce", "combo_bounces", "hellhound_charge", "hellhound_hit", "hellhound_nap"]:
		_check(seen.has(key), "Sandbox did not emit " + key)
	game._refresh_model()
	_check(game.model.hellhound.state == "nap" and game.model.hellhound.stamina == 0
		and game.model.hellhound.nap_seconds_left >= 11.95, "Nap state does not expose the 12-second countdown")
	await process_frame
	_check_revive(game)
	await process_frame
	game.hitstop = 0.0
	game._effect(["combo_bounces", [234, 148], 3, true])
	_check(is_equal_approx(game.hitstop, 0.020), "Melon combo did not trigger 20 ms hitstop")

func _check_revive(game: Node) -> void:
	var bridge: RefCounted = game.bridge
	var dog: Vector2 = game._vec(game.model.hellhound.pos)
	var closest := 0
	for slot in range(1, 3):
		if absf(game._fruit_center(slot).x - dog.x) < absf(game._fruit_center(closest).x - dog.x):
			closest = slot
	var center: Vector2 = game._fruit_center(closest)
	var gap := dog.x - center.x
	var vx := clampf(gap * 3.0, -29.2, 29.2) if absf(gap) > 7.0 else 0.0
	_check(bridge.swap_plants(8, closest) and bridge.cut_fruit_angled(closest, vx), "Could not feed the reserved apple")
	for i in 30:
		bridge.advance(4 * STEP)
		var events := _events(bridge)
		if _has_event(events, "hellhound_revived"):
			game._refresh_model()
			for event in events:
				game._effect(event)
			_check(game.model.hellhound.stamina == 3 and game.model.hellhound.state == "patrol"
				and game.effects.any(func(effect): return effect.has("hearts")),
				"Apple revive did not refill stamina and play hearts")
			return
	_check(false, "Reserved ripe apple missed the napping companion at %s" % dog)
