// Seeds every function from the Xbox 360 PE exception table (.pdata) before
// auto-analysis, so Ghidra's function list matches the binary's own table.
//
// PPC RUNTIME_FUNCTION (8 bytes, big-endian):
//   u32 BeginAddress
//   u32 PrologLength:8 | FunctionLength:22 (instructions) | ThirtyTwoBit:1 | ExceptionFlag:1
//
// Also turns off "Decompiler Parameter ID", which is prohibitively slow on a
// 60k-function PowerPC image and is not needed for the source export.
//@category UnleashedRE
import ghidra.app.cmd.disassemble.DisassembleCommand;
import ghidra.app.cmd.function.CreateFunctionCmd;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressSet;
import ghidra.program.model.address.AddressSpace;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.symbol.SourceType;

import java.util.ArrayList;
import java.util.List;

public class XenonPdataFunctions extends GhidraScript {
    @Override
    public void run() throws Exception {
        try {
            setAnalysisOption(currentProgram, "Decompiler Parameter ID", "false");
        } catch (Exception e) {
            println("could not disable Decompiler Parameter ID: " + e.getMessage());
        }

        Memory mem = currentProgram.getMemory();
        MemoryBlock pdata = mem.getBlock(".pdata");
        if (pdata == null) {
            println("XenonPdataFunctions: no .pdata block; nothing seeded");
            return;
        }
        AddressSpace space = currentProgram.getAddressFactory().getDefaultAddressSpace();
        long entries = pdata.getSize() / 8;
        AddressSet code = new AddressSet();
        List<Address> starts = new ArrayList<>();
        List<AddressSet> bodies = new ArrayList<>();
        int outside = 0;
        for (long i = 0; i < entries; i++) {
            Address e = pdata.getStart().add(i * 8);
            long begin = mem.getInt(e) & 0xffffffffL;
            long data = mem.getInt(e.add(4)) & 0xffffffffL;
            long length = ((data >> 8) & 0x3FFFFFL) * 4;
            if (begin == 0 || length == 0) {
                continue;
            }
            Address start = space.getAddress(begin);
            Address end = start.add(length - 1);
            if (!mem.contains(start) || !mem.contains(end)) {
                outside++;
                continue;
            }
            AddressSet body = new AddressSet(start, end);
            code.add(body);
            starts.add(start);
            bodies.add(body);
        }
        println("XenonPdataFunctions: " + starts.size() + " .pdata entries (" + outside + " outside memory)");

        DisassembleCommand dis = new DisassembleCommand(code, code, true);
        dis.applyTo(currentProgram, monitor);

        int created = 0;
        int existing = 0;
        for (int i = 0; i < starts.size(); i++) {
            monitor.checkCancelled();
            Address start = starts.get(i);
            if (currentProgram.getFunctionManager().getFunctionAt(start) != null) {
                existing++;
                continue;
            }
            CreateFunctionCmd cmd = new CreateFunctionCmd(null, start, bodies.get(i), SourceType.ANALYSIS);
            if (cmd.applyTo(currentProgram, monitor)) {
                created++;
            }
        }
        println("XenonPdataFunctions: created " + created + " functions, " + existing + " already existed");
    }
}
