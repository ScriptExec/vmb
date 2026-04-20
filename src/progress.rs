use indicatif::ProgressStyle;

pub(crate) fn default_spinner_style() -> ProgressStyle {
	ProgressStyle::default_spinner()
		.template("{spinner:.cyan} {msg}")
		.expect("ProgressStyle template was not valid")
		.tick_chars("⠁⠂⠄⡀⡈⡐⡠⣀⣁⣂⣄⣌⣔⣤⣥⣦⣮⣶⣷⣿⡿⠿⢟⠟⡛⠛⠫⢋⠋⠍⡉⠉⠑⠡⢁")
}

pub(crate) fn default_progress_style() -> ProgressStyle {
	ProgressStyle::default_bar()
		.template("{msg}\n{spinner:.cyan} [{wide_bar:.white/white}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
		.expect("ProgressStyle template was not valid")
		.progress_chars("=> ")
		//.tick_chars("⠦⠇⠋⠙⠸⠴")
		.tick_chars("⠁⠂⠄⡀⡈⡐⡠⣀⣁⣂⣄⣌⣔⣤⣥⣦⣮⣶⣷⣿⡿⠿⢟⠟⡛⠛⠫⢋⠋⠍⡉⠉⠑⠡⢁")
}
