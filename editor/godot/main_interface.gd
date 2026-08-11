extends HSplitContainer

var file_list: Array[ALFile] = []

func _ready() -> void:
	get_viewport().files_dropped.connect(on_files_dropped)
	pass

func _process(delta: float) -> void:
	pass
	
func on_files_dropped(files):
	print(files)
	for file in files:
		file_list.push_back(ALFile.load(file))
	pass
