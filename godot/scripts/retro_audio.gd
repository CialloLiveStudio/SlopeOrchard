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
		"cut": _sample(0.075, 1200.0, 240.0, 0.8),
		"hit": _sample(0.11, 192.0, 64.0, 0.42),
		"bounce": _sample(0.19, 155.0, 615.0, 0.02),
		"ring": _sample(0.34, 1174.0, 1172.0, 0.0),
		"bowl": _sample(0.29, 105.0, 48.0, 0.61),
	}
	for i in 8:
		var player := AudioStreamPlayer.new()
		player.volume_db = -15.0
		add_child(player)
		players.append(player)

func play(key: String) -> void:
	if muted:
		return
	var player := players[voice]
	voice = (voice + 1) % players.size()
	player.stream = sounds[key]
	player.play()

func _sample(duration: float, first: float, last: float, grain: float) -> AudioStreamWAV:
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
		var signal_value := lerpf(tone, rng.randf_range(-1.0, 1.0), grain)
		data.encode_s16(i * 2, int(signal_value * envelope * 26000.0))
	var stream := AudioStreamWAV.new()
	stream.format = AudioStreamWAV.FORMAT_16_BITS
	stream.mix_rate = RATE
	stream.data = data
	return stream
