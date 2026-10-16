// Decompiles specific (very large) functions with a long timeout.
// args: <output_dir> <timeout_seconds> <address> [<address> ...]
// Writes <output_dir>/<address>.c for each function that decompiles.
//@category UnleashedRE
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;

import java.io.File;
import java.io.FileOutputStream;
import java.io.OutputStreamWriter;
import java.io.Writer;
import java.nio.charset.StandardCharsets;

public class DecompileLong extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        File out = new File(args[0]);
        out.mkdirs();
        int timeout = Integer.parseInt(args[1]);
        DecompInterface d = new DecompInterface();
        DecompileOptions options = new DecompileOptions();
        options.grabFromProgram(currentProgram);
        options.setMaxPayloadMBytes(512);
        options.setMaxInstructions(500000);
        d.setOptions(options);
        d.toggleCCode(true);
        d.toggleSyntaxTree(false);
        d.setSimplificationStyle("decompile");
        d.openProgram(currentProgram);
        for (int i = 2; i < args.length; i++) {
            Function f = getFunctionAt(toAddr(args[i]));
            if (f == null) {
                println("no function at " + args[i]);
                continue;
            }
            long t0 = System.currentTimeMillis();
            DecompileResults r = d.decompileFunction(f, timeout, monitor);
            long secs = (System.currentTimeMillis() - t0) / 1000;
            if (r.decompileCompleted()) {
                try (Writer w = new OutputStreamWriter(new FileOutputStream(new File(out, args[i] + ".c")),
                        StandardCharsets.UTF_8)) {
                    w.write(r.getDecompiledFunction().getC());
                }
                println("DecompileLong: " + args[i] + " ok in " + secs + "s");
            } else {
                println("DecompileLong: " + args[i] + " failed after " + secs + "s: " + r.getErrorMessage());
            }
        }
        d.dispose();
    }
}
