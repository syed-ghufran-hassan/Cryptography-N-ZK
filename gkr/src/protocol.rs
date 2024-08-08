use crate::{
    interfaces::GKRProtocolInterface,
    primitives::GKRProof,
    utils::{gen_w_mle, perform_gkr_sumcheck_layer_one, verifiy_gkr_sumcheck_layer_one},
};
use ark_ff::PrimeField;
use circuits::{
    interfaces::GKRProtocolCircuitInterface,
    primitives::{Circuit, CircuitEvaluation},
};
use fiat_shamir::{interface::TranscriptInterface, FiatShamirTranscript};
use polynomial::{
    composed::multilinear::ComposedMultilinear, interface::MultilinearPolynomialInterface,
};
use sum_check::{
    composed::multicomposed::{MultiComposedProver, MultiComposedVerifier},
    interface::{MultiComposedProverInterface, MultiComposedVerifierInterface},
};

pub struct GKRProtocol;

impl<F: PrimeField> GKRProtocolInterface<F> for GKRProtocol {
    fn prove(circuit: &Circuit, evals: &CircuitEvaluation<F>) -> GKRProof<F> {
        let mut transcript = FiatShamirTranscript::new(vec![]);

        let mut sum_check_proofs = vec![];
        let mut w_i_b = vec![];
        let mut w_i_c = vec![];

        let w_0_mle = gen_w_mle(&evals.layers, 0);
        transcript.append(w_0_mle.to_bytes());

        let mut n_r = transcript.sample_n_as_field_elements(w_0_mle.num_vars);
        let mut claim = w_0_mle.evaluate(&n_r).unwrap();

        let mut last_rand_b = Vec::<F>::new();
        let mut last_rand_c = Vec::<F>::new();
        let mut last_alpha = F::zero();
        let mut last_beta = F::zero();

        // Running sumcheck on layer one
        let (add_mle_layer_one, mul_mle_layer_one) = circuit.get_add_n_mul_mle::<F>(0);
        let w_1_mle = gen_w_mle(&evals.layers, 1);
        let (layer_one_claim, layer_one_rand_b, layer_one_rand_c, layer_one_alpha, layer_one_beta) =
            perform_gkr_sumcheck_layer_one(
                claim,
                n_r.clone(),
                &add_mle_layer_one,
                &mul_mle_layer_one,
                &w_1_mle,
                &mut transcript,
                &mut sum_check_proofs,
                &mut w_i_b,
                &mut w_i_c,
            );

        claim = layer_one_claim;
        last_rand_b = layer_one_rand_b;
        last_rand_c = layer_one_rand_c;
        last_alpha = layer_one_alpha;
        last_beta = layer_one_beta;

        // starting the GKR round reductions powered by sumcheck (layer 2 to n-1(excluding the input layer))
        for l_index in 2..evals.layers.len() {
            let (add_mle, mul_mle) = circuit.get_add_n_mul_mle::<F>(l_index - 1);
            let w_i_mle = gen_w_mle(&evals.layers, l_index);

            let number_of_round = layer_one_rand_b.len();

            // add(r_b, b, c) ---> add(b, c)
            let add_r_b_c =
                add_mle.partial_evaluations(layer_one_rand_b.clone(), vec![0; number_of_round]);
            // mul(r_b, b, c) ---> mul(b, c)
            let mul_b_c =
                mul_mle.partial_evaluations(layer_one_rand_b.clone(), vec![0; number_of_round]);

            // add(r_c, b, c) ---> add(b, c)
            let add_r_c =
                add_mle.partial_evaluations(layer_one_rand_c.clone(), vec![0; number_of_round]);
            // mul(r_c, b, c) ---> mul(b, c)
            let mul_b_c =
                mul_mle.partial_evaluations(layer_one_rand_c.clone(), vec![0; number_of_round]);

            // let alpha_beta_add_b_c =

            let wb = w_i_mle.clone();
            let wc = w_i_mle.clone();

            // w_i(b) + w_i(c)
            let wb_add_wc = wb.add_distinct(&wc);
            // w_i(b) * w_i(c)
            let wb_mul_wc = wb.mul_distinct(&wc);

            //  add(b, c)(w_i(b) + w_i(c))
            let f_b_c_add_section = ComposedMultilinear::new(vec![add_b_c, wb_add_wc]);
            // mul(b, c)(w_i(b) * w_i(c))
            let f_b_c_mul_section = ComposedMultilinear::new(vec![mul_b_c, wb_mul_wc]);

            // f(b, c) = add(r, b, c)(w_i(b) + w_i(c)) + mul(r, b, c)(w_i(b) * w_i(c))
            let f_b_c = vec![f_b_c_add_section, f_b_c_mul_section];

            // this prover that the `claim` is the result of the evalution of the preivous layer
            let (sumcheck_proof, random_challenges) =
                MultiComposedProver::sum_check_proof_without_initial_polynomial(&f_b_c, &claim);

            transcript.append(sumcheck_proof.to_bytes());
            sum_check_proofs.push(sumcheck_proof);

            let (rand_b, rand_c) = random_challenges.split_at(random_challenges.len() / 2);

            let eval_w_i_b = wb.evaluate(&rand_b.to_vec()).unwrap();
            let eval_w_i_c = wc.evaluate(&rand_c.to_vec()).unwrap();

            w_i_b.push(eval_w_i_b);
            w_i_c.push(eval_w_i_c);

            // TODO: perform mathematical proof bindings for the eval_w_i_b and eval_w_i_c

            // n_r = transcript.sample_n_as_field_elements(w_i_mle.num_vars);
            // claim = w_i_mle.evaluate(&n_r).unwrap();
        }

        GKRProof {
            sum_check_proofs,
            w_i_b,
            w_i_c,
            w_0_mle,
        }
    }

    fn verify(circuit: &Circuit, input: &[F], proof: &GKRProof<F>) -> bool {
        // performing some sanity checks
        if proof.sum_check_proofs.len() != proof.w_i_b.len()
            || proof.sum_check_proofs.len() != proof.w_i_c.len()
        {
            println!("Invalid GKR proof");
            return false;
        }

        let mut transcript = FiatShamirTranscript::default();
        transcript.append(proof.w_0_mle.to_bytes());

        let mut n_r = transcript.sample_n_as_field_elements(proof.w_0_mle.num_vars);
        let mut claim = proof.w_0_mle.evaluate(&n_r).unwrap();

        // layer one verification logic
        let (add_mle, mul_mle) = circuit.get_add_n_mul_mle::<F>(0);
        if !verifiy_gkr_sumcheck_layer_one(
            &claim,
            &proof.sum_check_proofs[0],
            &mut transcript,
            proof.w_i_b[0],
            proof.w_i_c[0],
            n_r,
            &add_mle,
            &mul_mle,
        ) {
            return false;
        }

        // for i in 0..proof.sum_check_proofs.len() {
        //     if proof.sum_check_proofs[i].sum != claim {
        //         println!("Invalid sumcheck proof");
        //         return false;
        //     }

        //     transcript.append(proof.sum_check_proofs[i].to_bytes());
        //     let intermidate_claim_check =
        //         MultiComposedVerifier::verify_except_last_check(&proof.sum_check_proofs[i]);

        //     // performing sum check last check
        //     let (rand_b, rand_c) = intermidate_claim_check
        //         .random_challenges
        //         .split_at(intermidate_claim_check.random_challenges.len() / 2);
        //     let (add_mle, mul_mle) = circuit.get_add_n_mul_mle::<F>(i);
        //     let mut r_b_c = n_r.clone();
        //     r_b_c.extend_from_slice(&intermidate_claim_check.random_challenges);

        //     let w_b = proof.w_i_b[i];
        //     let w_c = proof.w_i_c[i];

        //     let add_b_c = add_mle.evaluate(&r_b_c).unwrap();
        //     let mul_b_c = mul_mle.evaluate(&r_b_c).unwrap();

        //     let add_section = add_b_c * (w_b + w_c);
        //     let mul_section = mul_b_c * (w_b * w_c);

        //     let f_b_c_eval = add_section + mul_section;

        //     if f_b_c_eval != intermidate_claim_check.claimed_sum {
        //         println!("Invalid sumcheck proof");
        //         return false;
        //     }

        //     // n_r = transcript.sample_n_as_field_elements(w_i_mle.num_vars);
        //     // claim =

        //     break;
        // }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_test_curves::bls12_381::Fr;
    use circuits::{
        interfaces::CircuitInterface,
        primitives::{CircuitLayer, Gate, GateType},
    };

    // sample circuit evaluation
    //      100(*)    - layer 0
    //     /     \
    //   5(+)_0   20(*)_1 - layer 1
    //   / \    /  \
    //  2   3   4   5
    #[test]
    fn test_gkr_protocol() {
        let layer_0 = CircuitLayer::new(vec![Gate::new(GateType::Mul, [0, 1])]);
        let layer_1 = CircuitLayer::new(vec![
            Gate::new(GateType::Add, [0, 1]),
            Gate::new(GateType::Mul, [2, 3]),
        ]);
        let circuit = Circuit::new(vec![layer_0, layer_1]);
        let input = [
            Fr::from(2u32),
            Fr::from(3u32),
            Fr::from(4u32),
            Fr::from(5u32),
        ];
        let evaluation = circuit.evaluate(&input);

        let proof = GKRProtocol::prove(&circuit, &evaluation);

        assert!(GKRProtocol::verify(&circuit, &input, &proof));
    }
}
