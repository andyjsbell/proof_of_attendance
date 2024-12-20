use ark_bn254::Bn254;
use ark_bn254::Fr;
use ark_ff::PrimeField;
use ark_groth16::Groth16;
use ark_r1cs_std::{fields::fp::FpVar, prelude::*};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_snark::SNARK;
use rand::thread_rng;

#[derive(Clone)]
struct SumCircuit<F: PrimeField> {
    pub a: Option<F>,
    pub b: Option<F>,
    pub c: Option<F>,
}

impl<F: PrimeField> ConstraintSynthesizer<F> for SumCircuit<F> {
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
        // Create variables for our private inputs (a and b)
        // c ==> a + b
        let a_var = FpVar::new_witness(cs.clone(), || {
            self.a.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let b_var = FpVar::new_witness(cs.clone(), || {
            self.b.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let c_var = FpVar::new_input(cs.clone(), || {
            self.c.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Add them together
        let sum = a_var + b_var;
        // Enforce that their sum equals the public output
        sum.enforce_equal(&c_var)?;

        Ok(())
    }
}

#[derive(Clone, Default)]
struct CompareCircuit<F: PrimeField> {
    pub shorter: Option<Vec<F>>,
    pub larger: Option<Vec<F>>,
}

impl<F: PrimeField> ConstraintSynthesizer<F> for CompareCircuit<F> {
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
        let shorter = self.shorter.ok_or(SynthesisError::AssignmentMissing)?;
        let larger = self.larger.ok_or(SynthesisError::AssignmentMissing)?;

        if shorter.len() > larger.len() {
            return Err(SynthesisError::Unsatisfiable);
        }

        // Public
        let shorter_vars = shorter
            .iter()
            .map(|&val| FpVar::new_input(cs.clone(), || Ok(val)))
            .collect::<Result<Vec<_>, _>>()?;

        // Witness
        let larger_vars = larger
            .iter()
            .take(shorter.len())
            .map(|&val| FpVar::new_witness(cs.clone(), || Ok(val)))
            .collect::<Result<Vec<_>, _>>()?;

        for (shorter_var, larger_var) in shorter_vars.iter().zip(larger_vars.iter()) {
            larger_var.enforce_equal(shorter_var)?;
        }
        Ok(())
    }
}

fn prove_verify_sum(a: u64, b: u64, c: u64) {
    let circuit = SumCircuit {
        a: Some(a.into()),
        b: Some(b.into()),
        c: Some(c.into()),
    };
    let rng = &mut thread_rng();

    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), rng).unwrap();
    let proof = Groth16::<Bn254>::prove(&pk, circuit, rng).expect("proof");
    let public_input = [c.into()];
    let verified = Groth16::<Bn254>::verify(&vk, &public_input, &proof).expect("verified");

    assert!(verified, "this can't be verified");
}

// Generate a vector of prime field values for a string
#[derive(Clone)]
struct PrimeString<F: PrimeField>(Vec<F>);
impl<F: PrimeField> From<&'static str> for PrimeString<F> {
    fn from(value: &'static str) -> Self {
        Self(
            value
                .as_bytes()
                .iter()
                .map(|c| (*c as u64).into())
                .collect(),
        )
    }
}

impl<F: PrimeField> From<PrimeString<F>> for Vec<F> {
    fn from(value: PrimeString<F>) -> Self {
        value.0.clone()
    }
}

fn prove_verify_starts_with(small: &'static str, large: &'static str) {
    let larger_array: PrimeString<Fr> = large.into();
    let shorter_array: PrimeString<Fr> = small.into();

    let circuit = CompareCircuit {
        larger: Some(larger_array.into()),
        shorter: Some(shorter_array.clone().into()),
    };

    let rng = &mut thread_rng();

    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(circuit.clone(), rng).unwrap();
    let proof = Groth16::<Bn254>::prove(&pk, circuit, rng).expect("proof");
    let verified =
        Groth16::<Bn254>::verify(&vk, &Vec::<Fr>::from(shorter_array), &proof).expect("verified");

    assert!(verified, "this can't be verified");
}

fn main() {
    prove_verify_sum(3, 4, 7);
    prove_verify_starts_with("bad", "bcdef");
}
