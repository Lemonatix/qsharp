namespace Kata {
    import Std.Diagnostics.*;

    operation LearnBasisStateAmplitudes(qs : Qubit[]) : (Double, Double) {
        DumpMachine(); // Only used to learn the amplitudes.
        return (0.3390, 0.3821);
    }
}
