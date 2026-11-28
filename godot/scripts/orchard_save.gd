extends RefCounted

const PATH := "user://slope_orchard_save.json"
const WEB_KEY := "slope_orchard_save_v1"

var journal: RefCounted = ClassDB.instantiate("OrchardJournal")
var data: Dictionary
var entries: Array
var persistent: bool
var path: String

func _init(use_storage := true, file_path := PATH) -> void:
	persistent = use_storage
	path = file_path
	if persistent:
		for candidate in [path, path + ".tmp"]:
			if FileAccess.file_exists(candidate):
				journal.restore(FileAccess.get_file_as_string(candidate))
		if OS.has_feature("web"):
			var raw = JavaScriptBridge.eval("try {localStorage.getItem('%s')} catch(e) {null}" % WEB_KEY, true)
			if raw is String:
				journal.restore(raw)
	entries = JSON.parse_string(journal.entries())
	data = JSON.parse_string(journal.encode())
	if persistent and data.revision > 0:
		commit(true)

func commit(changed: bool) -> void:
	if not changed:
		return
	var raw: String = journal.encode()
	data = JSON.parse_string(raw)
	if not persistent:
		return
	if OS.has_feature("web"):
		var ok = JavaScriptBridge.eval("try {localStorage.setItem('%s', %s); true} catch(e) {false}" % [WEB_KEY, JSON.stringify(raw)], true)
		if not ok:
			push_warning("Browser storage unavailable")
	var file := FileAccess.open(path + ".tmp", FileAccess.WRITE)
	if file == null:
		push_warning("Cannot open orchard save")
		return
	var written := file.store_string(raw)
	file.close()
	if not written or DirAccess.rename_absolute(path + ".tmp", path) != OK:
		push_warning("Could not commit orchard save")

func collected(id: int) -> bool:
	# JSON.parse_string stores JSON numbers as floats. Array.has is type-sensitive.
	return float(id) in data.fruits or float(id) in data.companions

func continue_stage() -> String:
	for id in ["t1", "t2", "t5"]:
		if not data.clears.has(id):
			return id
	return "t5"

