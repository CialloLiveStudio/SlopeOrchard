extends Node

const RATE := 22050
var muted := false:
	set(value):
		muted = value
		if muted:
			for player in players:
				player.stop()
		if music != null:
			if muted:
				music.stop()
			else:
				_start_music()
var sounds := {}
var players: Array[AudioStreamPlayer] = []
var voice := 0
var music: AudioStreamPlayer
var scores := {}
var cue := ""
var unlocked := not OS.has_feature("web")

func _ready() -> void:
	sounds = {
		"synthesis": _sample(0.21, 830.0, 1660.0, 0.0, true),
		"quake": _sample(0.24, 115.0, 37.0, 0.32),
		"pop": _sample(0.095, 660.0, 145.0, 0.04),
		"sticky_roll": _sample(0.29, 147.0, 78.0, 0.14),
		"sticky_catch": _sample(0.09, 370.0, 98.0, 0.03),
		"bubble_swallow": _sample(0.19, 104.0, 355.0, 0.01),
		"apple_rain": _phrase([77,84,81,76,72],0.042,false),
		"candy_land": _phrase([76,79,86],0.09,false),
		"pinball": _phrase([86,93],0.039,false),
		"cut": _sample(0.075, 1200.0, 240.0, 0.8),
		"green_cut": _sample(0.041, 2100.0, 520.0, 0.36),
		"hit": _sample(0.11, 192.0, 64.0, 0.42),
		"bounce": _sample(0.085, 510.0, 790.0, 0.06),
		"ring": _sample(0.34, 1174.0, 1172.0, 0.0),
		"bowl": _sample(0.29, 105.0, 48.0, 0.61),
		"heal": _sample(0.26, 480.0, 1120.0, 0.0),
		"bonk": _sample(0.15, 650.0, 210.0, 0.05),
		"home_run": _sample(0.095, 850.0, 125.0, 0.57),
		"shield_lid_dent": _sample(0.27, 510.0, 505.0, 0.07, true),
		"shield_lid_break": _sample(0.46, 835.0, 245.0, 0.22, true),
		"munch": _sample(0.13, 420.0, 260.0, 0.28),
		"sour": _sample(0.19, 540.0, 180.0, 0.09),
		"charge": _sample(0.10, 170.0, 100.0, 0.25),
	}
	for i in 8:
		var player := AudioStreamPlayer.new()
		player.volume_db = -15.0
		add_child(player)
		players.append(player)
	music = AudioStreamPlayer.new()
	music.volume_db = -18.5
	add_child(music)
	scores["pastoral"] = _phrase([72,0,76,79,0,76,74,0,69,72,76,0,74,71,67,0,72,76,79,83,81,0,76,74,72,0,69,74,71,67,72,0],0.32,true)
	scores["battle"] = _phrase([69,76,0,79,76,81,79,76,74,77,81,0,77,74,72,71,69,72,76,81,84,83,81,79,77,74,71,74,76,0,71,68],0.175,true,true)
	scores["victory"] = _phrase([72,76,79,0,84,83,84,0],0.19,false)
	sounds["tame"] = scores["victory"]
	scores["dusk"] = _phrase([69,0,65,0,64,0,60,0],0.24,false)

func unlock() -> void:
	# Godot resumes its Web Audio context during the canvas input gesture.
	if not unlocked:
		unlocked = true
		_start_music()

func score(key: String) -> void:
	if cue != key:
		cue = key
		_start_music()

func _start_music() -> void:
	if unlocked and not muted and scores.has(cue):
		music.stream = scores[cue]
		music.play()

func _phrase(notes: Array, beat: float, repeat: bool, lively := false) -> AudioStreamWAV:
	const MIX_RATE := 11025
	var length := roundi(beat*MIX_RATE)
	var data := PackedByteArray()
	data.resize(notes.size()*length*2)
	var roots := [48,53,45,55] if not lively else [45,50,45,52]
	for step in notes.size():
		var note: int = notes[step]
		var lead := TAU*440*pow(2.0,(note-69)/12.0)
		var bass := TAU*440*pow(2.0,(int(roots[(step/8)%4])-69)/12.0)
		for i in length:
			var t := float(i)/MIX_RATE
			var env := minf(t/0.012,1)*pow(1.0-float(i)/length,1.5)
			var signal_value := 0.0
			if note > 0:
				signal_value = 0.49*sin(t*lead)+0.10*sin(3*t*lead)+0.04*sin(2*t*lead)
			if repeat:
				signal_value += (0.24 if step%2 == 0 else 0.12)*sin(t*bass*(1.0 if step%2 == 0 else 1.5))
			if lively and step%2 == 0:
				signal_value += 0.22*sin(t*TAU*(65+48*exp(-t*42)))*exp(-t*33)
			data.encode_s16(2*(step*length+i),roundi(31000*env*signal_value))
	var stream := AudioStreamWAV.new()
	stream.format = AudioStreamWAV.FORMAT_16_BITS
	stream.mix_rate = MIX_RATE
	stream.data = data
	if repeat:
		stream.loop_end = length*notes.size()
		stream.loop_mode = AudioStreamWAV.LOOP_FORWARD
	return stream

func play_combo(bounces: int, pinball: bool) -> void:
	play("pinball" if pinball else "bounce", pow(2.0, 2.0 * clampi(bounces - 1, 0, 8) / 12.0))

func play(key: String, pitch := 1.0) -> void:
	if muted:
		return
	var player := players[voice]
	voice = (voice + 1) % players.size()
	player.stream = sounds[key]
	player.pitch_scale = pitch
	player.play()

func _sample(duration: float, first: float, last: float, grain: float, metallic := false) -> AudioStreamWAV:
	var frames := int(duration * RATE)
	var data := PackedByteArray()
	data.resize(frames * 2)
	var phase := 0.0
	var rng := RandomNumberGenerator.new()
	rng.seed = 10005
	for i in frames:
		var progress := float(i) / frames
		phase += lerpf(first, last, progress) * TAU / RATE
		var envelope := minf(1.0, progress * 80.0) * pow(1.0 - progress, 2.6)
		var tone := sin(phase) * 0.73 + signf(sin(phase * 2.0)) * 0.23
		if metallic:
			tone = sin(phase) * 0.42 + sin(phase * 2.76) * 0.32 + sin(phase * 4.07) * 0.21
		var signal_value := lerpf(tone, rng.randf_range(-1.0, 1.0), grain)
		data.encode_s16(i * 2, int(signal_value * envelope * 26000.0))
	var stream := AudioStreamWAV.new()
	stream.format = AudioStreamWAV.FORMAT_16_BITS
	stream.mix_rate = RATE
	stream.data = data
	return stream
