extends SceneTree

# Run with --fixed-fps 30 and a rendering display. Each cut goes through OrchardBridge.
const GAME = preload("res://scenes/main.tscn")
const NAMES := ["katamari","pinata","quake","bubble","pinball","bait"]
const FRAMES := [549,234,871,224,179,289]
const ENGLISH := ["Katamari","Piñata","Quake","Swallow Bubble","Pinball","Candied Bait"]
var game: Node
var destination: String
var caption: Node2D
var kind := 8
var captured := 0

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	if DisplayServer.get_name() == "headless":
		push_error("Showcase requires a rendering display")
		quit(1)
		return
	destination = ProjectSettings.globalize_path("res://../target/showcase_v05").simplify_path()
	if DirAccess.make_dir_recursive_absolute(destination) != OK:
		quit(1)
		return
	game = GAME.instantiate()
	game.persistent = false
	root.gui_disable_input = true
	root.add_child(game)
	game.set_process(false)
	game.set_process_unhandled_input(false)
	game.mouse = Vector2(-10000,-10000)
	for i in 55:
		game._process(1.0/30)
	await _capture("01_title")
	game.save.commit(game.save.journal.record_clear("t1",4))
	game.shell.buttons.stages.pressed.emit()
	await _capture("02_stage_select")
	game.start_stage("sandbox")
	for id in [8,9,11]:
		_plant(id)
	game._home()
	game.shell.open("Almanac")
	game.shell._select_group(2)
	await _capture("03_almanac")
	caption = Node2D.new()
	caption.draw.connect(_caption)
	game.add_child(caption)
	game.audio.muted = true
	for i in NAMES.size():
		kind = 8+i
		if not await _toy(FRAMES[i]):
			push_error("Expected action missing for "+NAMES[i])
			quit(1)
			return
		await _capture("%02d_toy_%s" % [4+i,NAMES[i]])
	game.queue_free()
	await process_frame
	print("Showcase: %d / 9 rendered PNGs" % captured)
	quit(0 if captured == 9 else 1)

func _plant(id: int) -> void:
	var recipe: Array = game.save.entries[id].recipe
	game.bridge.equip_plant(2,int(recipe[0]))
	game.bridge.drop_seed(int(recipe[1]),2)
	game._refresh_model()
	game._drain_events()

func _toy(frames: int) -> bool:
	game.world.scale = Vector2.ONE
	game.start_stage("sandbox")
	game.hud.hide()
	game.mute_button.hide()
	if kind == 12:
		game.bridge.remove_plant(1)
	game.bridge.set_hellhound(kind == 8)
	if kind != 8:
		game.bridge.set_lulucy(false)
	_plant(kind)
	game.elapsed = 0
	for frame in range(1,frames+1):
		if kind == 8:
			for substep in range(1,5):
				game.bridge.advance(1.0/120)
				if ((frame-1)*4 + substep)%507 == 0:
					game.bridge.cut_fruit(2)
		else:
			game.bridge.advance(1.0/30)
		game.elapsed = frame/30.0
		game._refresh_model()
		game.effects = game.effects.filter(func(e): return e.until > game.elapsed)
		for id in game.flashes.keys():
			if game.flashes[id] < game.elapsed:
				game.flashes.erase(id)
		if frame > frames-22:
			game._drain_events()
		else:
			game.bridge.take_events()
		if kind != 8 and game.model.slots[2].stage == 2:
			game.bridge.cut_fruit(2)
		if kind == 9:
			for body in game.model.bodies:
				if body.kind == 9 and body.pos[1] > 150:
					game.bridge.trigger_home_run(body.pos[0],body.pos[1])
		if frame > frames-22:
			await process_frame
	var focus := _action_position()
	if focus == Vector2.ZERO:
		return false
	var zoom := 1.7 if kind in [9,10,12] else 2.0
	game.world.scale = Vector2.ONE*zoom
	game.world.position = Vector2(clampf(240-focus.x*zoom,480-480*zoom,0),
		clampf(145-focus.y*zoom,270-270*zoom,0)).round()
	caption.queue_redraw()
	return true

func _action_position() -> Vector2:
	var m: Dictionary = game.model
	match kind:
		8,11:
			for b in m.bodies:
				if b.kind != kind:
					continue
				if kind == 8 and b.rolling and b.rotation > 0 and b.passengers.size() >= 2:
					return game._vec(b.pos)
				if kind == 11 and b.passengers.size() == 1 and b.velocity[1] < 0 and b.pos[1] < game._ground(b.pos[0])-46:
					return game._vec(b.pos)
		9:
			var rain: Array = m.bodies.filter(func(b): return b.kind == 0 and b.velocity[1] < 0)
			if rain.size() >= 5 and game.effects.any(func(e): return e.has("rain")):
				return _middle(rain) + Vector2(0,18)
		10:
			for e in game.effects:
				if e.has("quake"):
					var lifted: Array = m.monsters.filter(func(enemy): return game._ground(enemy.pos[0])-enemy.pos[1] > 30 and absf(enemy.pos[0]-e.pos.x)<e.quake+10)
					if lifted.size() >= 2:
						return (e.pos+_middle(lifted))/2
		12:
			var marbles: Array = m.bodies.filter(func(b): return b.kind == 12 and b.bounces > 0 and b.pos[1] < 170)
			if marbles.size() >= 3:
				return _middle(marbles)
		13:
			for bait in m.baits:
				var diners: Array = m.monsters.filter(func(e): return e.hungry and absf(e.pos[0]-bait.pos[0])<42)
				if diners.size() >= 3 and bait.bites < 8:
					return _middle(diners)
	return Vector2.ZERO

func _middle(items: Array) -> Vector2:
	var total := Vector2.ZERO
	for item in items:
		total += game._vec(item.pos)
	return total/items.size()

func _caption() -> void:
	var entry: Dictionary = game.save.entries[kind]
	caption.draw_rect(Rect2(0,0,480,32),game.INK)
	caption.draw_rect(Rect2(0,31,480,1),game.GOLD)
	caption.draw_texture_rect(game.fruit_textures[kind],Rect2(11,3,25,25),false)
	caption.draw_string(game.FONT,Vector2(43,21),entry.name,HORIZONTAL_ALIGNMENT_LEFT,-1,15,game.PAPER)
	caption.draw_string(game.FONT,Vector2(150,20),ENGLISH[kind-8],HORIZONTAL_ALIGNMENT_LEFT,-1,12,game.GOLD)
	for i in 2:
		caption.draw_texture_rect(game.fruit_textures[int(entry.recipe[i])],Rect2(413+i*37,5,20,20),false)
	caption.draw_string(game.FONT,Vector2(438,20),"+",HORIZONTAL_ALIGNMENT_LEFT,-1,12,game.PAPER)
	caption.draw_rect(Rect2(12,239,14+entry.note.length()*12,22), game.INK)
	caption.draw_string(game.FONT,Vector2(19,255),entry.note,HORIZONTAL_ALIGNMENT_LEFT,-1,12,game.PAPER)

func _capture(stem: String) -> void:
	game.world.queue_redraw()
	await process_frame
	await RenderingServer.frame_post_draw
	var frame := root.get_texture().get_image()
	var file := destination.path_join(stem+".png")
	if frame == null or frame.is_empty() or frame.save_png(file) != OK:
		push_error("Capture failed: "+file)
		return
	var check := Image.load_from_file(file)
	if check != null and check.get_size() == frame.get_size() and FileAccess.get_size(file) > 0:
		captured += 1
		print(stem, " ", check.get_size(), " ", FileAccess.get_size(file), " bytes")

