extends Node

const RATE := 22050
var muted := false:
	set(value):
		muted = value
		if muted:
			for player in players:
				player.stop()
var sounds := {}
var players: Array[AudioStreamPlayer] = []
var voice := 0

func _ready() -> void:
	sounds = {
		"synthesis": _sample(0.21, 830.0, 1660.0, 0.0, true),
		"quake": _sample(0.24, 115.0, 37.0, 0.32),
		"pop": _sample(0.095, 660.0, 145.0, 0.04),
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

func play_combo(bounces: int) -> void:
	play("bounce", pow(2.0, 2.0 * clampi(bounces - 1, 0, 8) / 12.0))

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
