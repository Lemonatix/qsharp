namespace Kata.Verification {
    import Std.Katas.*;
    import Std.Math.*;

    operation GlobalPhaseI(q : Qubit) : Unit is Adj + Ctl {
        X(q);
        Z(q);
        Y(q);
    }

    operation CheckSolution() : Bool {
        let solution = register => Kata.GlobalPhaseI(register[0]);
        let reference = register => GlobalPhaseI(register[0]);
        let isCorrect = CheckOperationsAreEqualStrict(1, solution, reference);

        // Output different feedback to the user depending on whether the solution was correct.
        if isCorrect {
            Message("Correct!");
        } else {
            Message("Incorrect.");
            Message("Hint: examine the effect your solution has on the state 0.6|0〉 + 0.8|1〉 and compare it with the effect it " +
                "is expected to have.");
            ShowQuantumStateComparison(1, qs => Ry(ArcTan2(0.8, 0.6) * 2.0, qs[0]), solution, reference);
        }
        isCorrect
    }
}
