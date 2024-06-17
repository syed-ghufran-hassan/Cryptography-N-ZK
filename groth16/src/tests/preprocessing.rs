use crate::{interfaces::R1CSProcessingInterface, preprocessing::PreProcessor, primitives::{QAPPolysCoefficients, Witness, R1CS}};
use ark_test_curves::bls12_381::Fr;

#[test]
fn test_to_qap_poly_coefficients() {
    let r1cs = R1CS::<Fr> {
        a: vec![
            vec![Fr::from(2u32), Fr::from(1u32)],
            vec![Fr::from(2u32), Fr::from(5u32)],
            vec![Fr::from(2u32), Fr::from(5u32)],
            vec![Fr::from(2u32), Fr::from(5u32)],
        ],
        b: vec![
            vec![Fr::from(2u32), Fr::from(2u32)],
            vec![Fr::from(2u32), Fr::from(2u32)],
            vec![Fr::from(2u32), Fr::from(2u32)],
            vec![Fr::from(2u32), Fr::from(2u32)],
        ],
        c: vec![
            vec![Fr::from(2u32), Fr::from(2u32)],
            vec![Fr::from(2u32), Fr::from(2u32)],
            vec![Fr::from(2u32), Fr::from(2u32)],
            vec![Fr::from(2u32), Fr::from(2u32)],
        ],
    };
    
    let preprocessor = PreProcessor::new(r1cs, Witness::new(vec![], vec![]));
    
    let qap_poly_coefficients = preprocessor.to_qap_poly_coefficients();
    
    let excpected_result = QAPPolysCoefficients {
        a: vec![
            vec![Fr::from(2u32), Fr::from(2u32), Fr::from(2u32), Fr::from(2u32)],
            vec![Fr::from(1u32), Fr::from(5u32), Fr::from(5u32), Fr::from(5u32)],
        ],
        b: vec![
            vec![Fr::from(2u32), Fr::from(2u32), Fr::from(2u32), Fr::from(2u32)],
            vec![Fr::from(2u32), Fr::from(2u32), Fr::from(2u32), Fr::from(2u32)],
        ],
        c: vec![
            vec![Fr::from(2u32), Fr::from(2u32), Fr::from(2u32), Fr::from(2u32)],
            vec![Fr::from(2u32), Fr::from(2u32), Fr::from(2u32), Fr::from(2u32)],
        ],
    };
    
    
    assert_eq!(qap_poly_coefficients.a, excpected_result.a);
    assert_eq!(qap_poly_coefficients.b, excpected_result.b);
    assert_eq!(qap_poly_coefficients.c, excpected_result.c);
}
