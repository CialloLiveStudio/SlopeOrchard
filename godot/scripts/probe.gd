extends SceneTree

const SCENE = preload("res://scenes/main.tscn")
const STEP := 1.0 / 120.0
const SaveStore = preload("res://scripts/orchard_save.gd")
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

func _run() -> void:
	_check_disk_save()
	var game := SCENE.instantiate()
	game.persistent = false
	root.add_child(game)
	await process_frame
	game.set_process(false)
	game.audio.muted = true
	_check(game.background.get_size() == Vector2(480, 270), "Pixel pipeline dimensions")
	_check(game.shell.page == "Title" and not game.hud.visible and game.mute_button.is_visible_in_tree(), "Initial Title")
	game.shell.buttons.stages.pressed.emit()
	_check(game.shell.page == "StageSelect" and game.shell.buttons.t2.disabled and game.shell.buttons.t5.disabled, "Roadmap gates")
	game.shell.buttons.test.pressed.emit()
	_check(not game.shell.buttons.t5.disabled, "Practice stage selection")
	game.shell.buttons.t5.pressed.emit()
	_check(game.practice_run and not game.save.collected(15) and not game.save.data.started, "Practice altered campaign")
	game.home_button.pressed.emit()
	game.shell.buttons.stages.pressed.emit()
	game.shell.buttons.test.pressed.emit()
	game.shell.buttons.other.pressed.emit()
	_check(game.shell.page == "Almanac" and game.save.entries.size() == 16 and game.save.data.fruits.size() == 4, "Default collection")
	game.shell.buttons.other.pressed.emit()
	game.shell.buttons.t1.pressed.emit()
	_check(game.model.stage == "Prep" and game.model.level_id == "t1", "Initial formal Prep")
	_check(game.model.preview == [0.0, 3.0] and game.next_button.text == "开始守卫", "T1 preview and start")
	await _check_green_input(game)
	_check_campaign(game)
	_check_grafts(game)
	await _check_live_cargo(game)
	_check_defeat(game)
	game._toggle_mute()
	_check(not game.audio.muted, "Audio control")
	game.audio.muted = true
	game.queue_free()
	await create_timer(0.15).timeout
	if failures == 0:
		print("Slope Orchard v0.5.0 probe passed: Title -> StageSelect -> Almanac -> Prep -> Battle, campaign records, practice isolation, Sandbox, 10 recipes, cargo, retries, companions and audio.")
	quit(0 if failures == 0 else 1)

func _check_disk_save() -> void:
	if OS.has_feature("web"):
		return
	var path := "user://probe_%d_%d.json" % [OS.get_process_id(),Time.get_ticks_usec()]
	var first := SaveStore.new(true,path)
	first.commit(first.journal.begin_campaign())
	first.commit(first.journal.record_clear("t1",4))
	first.commit(first.journal.discover(10))
	first.commit(first.journal.set_muted(true))
	_check(FileAccess.file_exists(path), "Desktop save missing")
	var restored := SaveStore.new(true,path)
	_check(restored.data == first.data and restored.collected(10) and restored.collected(14)
		and restored.continue_stage() == "t2" and restored.data.muted, "Desktop save round trip")
	DirAccess.remove_absolute(path)

func _scene_pixel(frame: Image, at: Vector2i) -> Color:
	return frame.get_pixelv(at * frame.get_size() / Vector2i(480, 270))

func _check_green_input(game: Node) -> void:
	var bridge: RefCounted = game.bridge
	_check(not bridge.start_level("unknown"), "Unknown level accepted")
	_check(not bridge.cut_fruit(-1) and not bridge.cut_fruit(1), "Invalid cuts accepted")
	_check(bridge.swap_plants(2, 1), "Could not set up neighboring vines")
	game.next_button.pressed.emit()
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
	game.next_button.pressed.emit()
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

func _check_campaign(game: Node) -> void:
	game._restart("t1")
	game.next_button.pressed.emit()
	var bridge: RefCounted = game.bridge
	var granted := false
	for level in ["t1", "t2"]:
		if level == "t2":
			_check(game.model.stage == "Prep" and game.companion_button.visible, "T2 companion selection")
			for i in game.slot_x.size():
				if game.model.slots[i] != null:
					bridge.equip_plant(i, 1)
			game.next_button.pressed.emit()
		var state: Dictionary
		for frame in 3400:
			state = _state(bridge)
			if state.stage == "Result":
				break
			if state.phase == game.REINFORCEMENT:
				game._refresh_model()
				var slot: int = game.model.slots.find(null)
				_drag(game, game.SEEDS + Vector2(game.SEED_STEP,0), game._fruit_center(slot), MOUSE_BUTTON_LEFT)
				_check(game.model.phase == game.WAVE and game.model.slots[slot].kind == 1, "Grant placement must resume Battle")
				granted = true
			elif level == "t1":
				_t1_cuts(bridge, state)
			else:
				for i in state.slots.size():
					var item = state.slots[i]
					if item != null and item.stage >= 1 and state.monsters.any(func(m): return absf(m.pos[0] - state.slot_x[i] - 11) < 24):
						bridge.cut_fruit(i)
			bridge.advance(0.10)
			bridge.take_events()
		game._refresh_model()
		_check(game.model.phase == game.VICTORY, "Campaign victory in " + level)
		_check(game.save.data.clears.get(level,0) == game.model.protagonist.hp, "Best hearts in " + level)
		game.next_button.pressed.emit()
	_check(granted and game.model.level_id == "t5" and game.model.stage == "Prep", "T2 to T5 Prep")

func _check_grafts(game: Node) -> void:
	var recipes := [[0,0,4],[1,1,5],[2,2,6],[3,3,7],[2,3,8],[0,2,9],[1,2,10],[1,3,11],[0,1,12],[0,3,13]]
	for mode in ["t5", "sandbox"]:
		game._restart(mode)
		for recipe in recipes:
			var expected: int = recipe[2]
			for reverse in [false,true]:
				var a: int = recipe[int(reverse)]
				var b: int = recipe[1-int(reverse)]
				var button := MOUSE_BUTTON_RIGHT if mode == "sandbox" else MOUSE_BUTTON_LEFT
				game.bridge.equip_plant(0,a)
				game.bridge.equip_plant(1,b)
				game._refresh_model()
				var target: Vector2 = game._fruit_center(1)
				var seed: Vector2 = game.SEEDS + Vector2(a*game.SEED_STEP,0)
				_mouse_button(game, seed if reverse else game._fruit_center(0), MOUSE_BUTTON_LEFT if reverse else button, true)
				game.mouse = target
				var hover = game._merge_preview(1)
				_check(hover != null and hover.kind == expected and not hover.name.is_empty(), "Missing preview in "+mode)
				_check(game.model.slots[1].kind == b, "Hover mutated the rack")
				_mouse_button(game,target,MOUSE_BUTTON_LEFT if reverse else button,false)
				var result: Dictionary = game.model.slots[1]
				_check(result.kind == expected and hover != null and result.name == hover.name, "Preview/commit mismatch in "+mode)
				_check(game.save.collected(expected), "Committed recipe missing from Almanac")
				_check(game.effects.any(func(e): return e.has("merge_slot") and e.merge_slot == 1), "Missing synthesis feedback")
				_drag(game, seed, target, MOUSE_BUTTON_LEFT)
				_check(game.model.slots[1].kind == expected, "Third seed overwrote a hybrid")
				_drag(game, target,target,MOUSE_BUTTON_RIGHT)
				_check(int(game.model.slots[1].kind) in [a,b], "Right-click split failed")
				_drag(game, target,seed,button)
				_check(game.model.slots[1] == null, "Return to seed tray failed")
	game._restart("t5")
	var donor: Vector2 = game._fruit_center(1)
	var base: Vector2 = game._fruit_center(2)
	_drag(game,donor,base,MOUSE_BUTTON_LEFT)
	_drag(game,game.SEEDS,donor,MOUSE_BUTTON_LEFT)
	_drag(game,base,donor,MOUSE_BUTTON_LEFT,true)
	_check(game.model.slots[1].kind == 11 and game.model.slots[2].kind == 0, "Shift drag swap")
	game.lulucy_button.pressed.emit()
	_check(not game.model.lulucy and game.model.hellhound.present, "Independent Lulucy toggle")
	game.next_button.pressed.emit()
	_check(not game.companion_button.visible and not game.lulucy_button.visible and game.home_button.visible, "Battle controls locked")
	_check(not game.bridge.swap_plants(1, 2) and not game.bridge.graft_plants(1, 2)
		and not game.bridge.drop_seed(0,2) and not game.bridge.split_or_clear(1)
		and not game.bridge.equip_plant(2, 3) and not game.bridge.set_lulucy(false), "Core Battle lock")
	_drag(game, donor, base, MOUSE_BUTTON_LEFT, true)
	_check(game.model.slots[1].kind == 11, "Battle drag moved a hybrid")
	var records: Dictionary = game.save.data.clears.duplicate()
	game.start_stage("t5",true)
	game.next_button.pressed.emit()
	var tiers := {}
	var oil := false
	var burst := false
	for i in 2400:
		var state := _state(game.bridge)
		if state.stage == "Result":
			break
		for slot in state.slots.size():
			var fruit = state.slots[slot]
			if fruit != null and fruit.stage > 0 and state.monsters.any(func(m): return absf(m.pos[0] - state.slot_x[slot] - 10) < 24):
				game.bridge.cut_fruit(slot)
		game.bridge.advance(0.1)
		var events := _events(game.bridge)
		oil = oil or _has_event(events, "oil")
		burst = burst or _has_event(events, "fragile")
		for body in state.bodies:
			if body.kind == 1:
				tiers["spent" if body.near_shatter else ("fresh" if body.energy_ratio == 1.0 else "cracked")] = true
	game._refresh_model()
	_check(game.model.phase == game.VICTORY, "T5 showcase victory")
	_check(game.save.data.clears == records, "Practice victory modified the campaign")
	_check(oil and burst and tiers.size() == 3, "T5 oil burst and all three melon snapshot tiers")
	game._restart("sandbox")
	game.companion_button.pressed.emit()
	_check(game.model.lulucy and not game.model.hellhound.present, "Sandbox independent companions")
func _check_live_cargo(game: Node) -> void:
	for recipe in [[2,3,8],[1,3,11]]:
		game._restart("sandbox")
		for slot in [0,1,2,4,5]:
			game.bridge.remove_plant(slot)
		game.bridge.equip_plant(4,recipe[0])
		game.bridge.drop_seed(recipe[1],4)
		var carried := false
		for frame in 400:
			game.bridge.advance(0.1)
			var state := _state(game.bridge)
			if state.slots[4].stage >= 1:
				game.bridge.cut_fruit(4)
			for body in state.bodies:
				if body.kind != recipe[2] or body.passengers.is_empty():
					continue
				var passenger: Dictionary = body.passengers[0]
				_check(passenger.has("kind") and not state.monsters.any(func(m): return m.id == passenger.id),
					"Carried monster must appear once in the snapshot")
				_check(body.radius > 0 and (body.kind != 8 or body.rotation > 0), "Missing carrier geometry")
				carried = true
				break
			if carried:
				game._refresh_model()
				await process_frame
				break
		_check(carried, "Missing live passenger for recipe %d" % recipe[2])

func _check_defeat(game: Node) -> void:
	game._restart("t2")
	game.companion_button.pressed.emit()
	game.bridge.equip_plant(0, 1)
	game.next_button.pressed.emit()
	for direct in [true, false]:
		for i in 1400:
			game.bridge.advance(0.25)
			var state := _state(game.bridge)
			if state.phase == game.REINFORCEMENT:
				game.bridge.equip_plant(state.slots.find(null), 1)
			if state.stage == "Result":
				break
		game._refresh_model()
		_check(game.model.phase == game.DEFEAT and game.retry_button.visible, "Defeat controls")
		if direct:
			game.next_button.pressed.emit()
		else:
			game.retry_button.pressed.emit()
		_check(game.model.stage == ("Battle" if direct else "Prep") and game.model.slots[0].kind == 1
			and game.model.protagonist.hp == 5 and not game.model.hellhound.present, "Retry restores chosen setup")
