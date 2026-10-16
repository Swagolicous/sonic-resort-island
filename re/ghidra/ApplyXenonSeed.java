// Apply XenonRecomp-derived function seeds to the current Ghidra program.
//
// Args:
//   <seed.jsonl> [--overwrite-names] [--no-create]
//
// The seed file is produced by BuildXenonSeed.ps1.  This script intentionally
// uses only JDK + Ghidra APIs so it remains runnable by `ghidra script run`.
//
// @category UnleashedRecomp

import ghidra.app.cmd.function.CreateFunctionCmd;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionManager;
import ghidra.program.model.symbol.SourceType;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public class ApplyXenonSeed extends GhidraScript {
    private static final Pattern ADDRESS =
        Pattern.compile("\"address_hex\"\\s*:\\s*\"(0x[0-9A-Fa-f]+)\"");
    private static final Pattern NAME =
        Pattern.compile("\"name\"\\s*:\\s*\"([A-Za-z_][A-Za-z0-9_]*)\"");
    private static final Pattern NAME_KIND =
        Pattern.compile("\"name_kind\"\\s*:\\s*\"([A-Za-z0-9_]+)\"");
    private static final Pattern EXPLICIT_SIZE =
        Pattern.compile("\"explicit_size\"\\s*:\\s*(null|[0-9]+)");

    private static String match(Pattern p, String line) {
        Matcher m = p.matcher(line);
        return m.find() ? m.group(1) : null;
    }

    private static boolean isDefaultGhidraName(String name) {
        return name == null ||
            name.matches("(?i)FUN_[0-9a-f]+") ||
            name.matches("(?i)LAB_[0-9a-f]+") ||
            name.matches("(?i)SUB_[0-9a-f]+");
    }

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length < 1) {
            throw new IllegalArgumentException(
                "usage: ApplyXenonSeed.java <seed.jsonl> [--overwrite-names] [--no-create]");
        }

        File seedFile = new File(args[0]).getAbsoluteFile();
        if (!seedFile.isFile()) {
            throw new IllegalArgumentException("seed file not found: " + seedFile);
        }

        boolean overwriteNames = false;
        boolean createMissing = true;
        for (int i = 1; i < args.length; i++) {
            if ("--overwrite-names".equals(args[i])) {
                overwriteNames = true;
            } else if ("--no-create".equals(args[i])) {
                createMissing = false;
            } else {
                throw new IllegalArgumentException("unknown argument: " + args[i]);
            }
        }

        FunctionManager fm = currentProgram.getFunctionManager();
        long rows = 0;
        long inMemory = 0;
        long existing = 0;
        long created = 0;
        long createFailed = 0;
        long renamed = 0;
        long preservedNames = 0;
        long syntheticNamesSkipped = 0;
        long malformed = 0;

        int tx = currentProgram.startTransaction("Apply XenonRecomp function seed");
        boolean commit = false;
        try (BufferedReader r = new BufferedReader(new FileReader(seedFile))) {
            String line;
            while ((line = r.readLine()) != null) {
                monitor.checkCancelled();
                if (line.trim().isEmpty()) {
                    continue;
                }

                rows++;
                String addressText = match(ADDRESS, line);
                String seedName = match(NAME, line);
                String nameKind = match(NAME_KIND, line);
                String explicitSizeText = match(EXPLICIT_SIZE, line);
                if (addressText == null || seedName == null || nameKind == null ||
                    explicitSizeText == null) {
                    malformed++;
                    continue;
                }

                Address address;
                try {
                    long offset = Long.parseUnsignedLong(addressText.substring(2), 16);
                    address = currentProgram.getAddressFactory()
                        .getDefaultAddressSpace().getAddress(offset);
                } catch (Exception e) {
                    malformed++;
                    continue;
                }

                if (!currentProgram.getMemory().contains(address)) {
                    continue;
                }
                inMemory++;

                Function f = fm.getFunctionAt(address);
                if (f != null) {
                    existing++;
                } else if (createMissing) {
                    // Prefer Ghidra's standard function creation command.  The explicit
                    // XenonRecomp size remains provenance in the seed; forcing a body
                    // here can destroy discontiguous bodies discovered by Ghidra.
                    CreateFunctionCmd cmd = new CreateFunctionCmd(address);
                    if (cmd.applyTo(currentProgram, monitor)) {
                        f = fm.getFunctionAt(address);
                        if (f != null) {
                            created++;
                        } else {
                            createFailed++;
                        }
                    } else {
                        createFailed++;
                    }
                }

                if (f == null) {
                    continue;
                }

                // Synthetic sub_<address> names add no semantic information over a
                // Ghidra default name, so do not turn them into USER_DEFINED labels.
                if ("synthetic_address".equals(nameKind)) {
                    syntheticNamesSkipped++;
                    continue;
                }

                String oldName = f.getName();
                boolean mayRename = overwriteNames || isDefaultGhidraName(oldName);
                if (mayRename && !seedName.equals(oldName)) {
                    try {
                        f.setName(seedName, SourceType.IMPORTED);
                        renamed++;
                    } catch (Exception e) {
                        println("rename failed at " + address + " to " + seedName +
                            ": " + e.getMessage());
                    }
                } else if (!seedName.equals(oldName)) {
                    preservedNames++;
                }
            }

            commit = true;
        } finally {
            currentProgram.endTransaction(tx, commit);
        }

        println("seed=" + seedFile.getAbsolutePath());
        println("rows=" + rows +
            " in_memory=" + inMemory +
            " existing=" + existing +
            " created=" + created +
            " create_failed=" + createFailed +
            " renamed=" + renamed +
            " preserved_names=" + preservedNames +
            " synthetic_names_skipped=" + syntheticNamesSkipped +
            " malformed=" + malformed);
    }
}
