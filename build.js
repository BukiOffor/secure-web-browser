const { exec } = require("child_process");
const util = require("util");
const execAsync = util.promisify(exec);
const { execa } = require("execa");
const fs = require("fs");

const config = {
	source: "mapper/target/release/mapper",
	destFolder: "src-tauri/bin/mapper",
};

async function runCommand(command, description) {
	console.log(`\n🔄 ${description}...`);
	try {
		const { stdout, stderr } = await execAsync(command);
		if (stdout) console.log(stdout);
		if (stderr) console.error(stderr);
		console.log(`✅ ${description} completed`);
	} catch (error) {
		console.error(`❌ ${description} failed:`, error.message);
		process.exit(1);
	}
}

async function rename() {
	let extension = "";
	let fileName = "mapper";
	if (process.platform === "win32") {
		extension = ".exe";
	}
	const rustInfo = (await execa("rustc", ["-vV"])).stdout;
	const targetTriple = /host: (\S+)/g.exec(rustInfo)[1];
	if (!targetTriple) {
		console.error("Failed to determine platform target triple");
	}

	// Check if binary file has been renamed already
	if (fs.existsSync(`src-tauri/bin/${fileName}${extension}`)) {
		console.log(
			`Renaming ${fileName}${extension} to ${fileName}-${targetTriple}${extension}`
		);
		fs.renameSync(
			`src-tauri/bin/${fileName}${extension}`,
			`src-tauri/bin/${fileName}-${targetTriple}${extension}`
		);
	} else {
		console.log(
			`binary File (${fileName}${extension}) does not exist, skipping rename.`
		);
	}
}

function copy() {
	try {
		// Add .exe extension on Windows
		const extension = process.platform === "win32" ? ".exe" : "";
		let source = config.source + extension;
		let dest = config.destFolder + extension;

		// Check if source binary exists
		if (!fs.existsSync(source)) {
			throw new Error(`Source binary not found: ${config.source}`);
		}
		fs.copyFileSync(source, dest);

		// Make it executable on Unix-like systems
		if (process.platform !== "win32") {
			fs.chmodSync(dest, "755");
		}

		console.log(`✅ Binary copied and renamed successfully!`);
		console.log(`   From: ${source}`);
		console.log(`   To:   ${dest}`);
	} catch (error) {
		console.error("❌ Error copying binary:", error.message);
		process.exit(1);
	}
}

async function main() {
	console.log("Running Build Command...........");
	await runCommand(
		"cd mapper && cargo build --release",
		"Building Side Car Binary"
	);
	console.log("Copying binary............");
	copy();
	console.log("Renaming binary............");
	await rename();
	console.log("\n🎉 Build pipeline completed successfully!");
}

main().then(() => {
	process.exit(0);
});
