#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod segundo_contrato {

    use primer_contrato::{PrimerContratoRef, Usuario, Rol};
    use ink::storage::Mapping;
    use ink::storage::StorageVec;
    use ink::prelude::vec::Vec;
    use ink::prelude::string::{String, ToString};

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct SegundoContrato {
        /// Stores a single `bool` value on the storage.
        marketplace: PrimerContratoRef,
    }
    
    impl SegundoContrato {
        /// Constructor that initializes the `bool` value to the given `init_value`.
        // In `basic_contract_ref/lib.rs`
        
        #[ink(constructor)]
        pub fn new(other_contract_code_hash: Hash) -> Self {

            let marketplace = PrimerContratoRef::new()
                .code_hash(other_contract_code_hash)
                .endowment(0)
                .salt_bytes([0xDE, 0xAD, 0xBE, 0xEF])
                .instantiate();

            Self { marketplace }
        }

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(Default::default())
        }

        #[ink(message)]
        pub fn nada(&self) -> u8{
            return 4
        }

        #[ink(message)]
        pub fn vendedores_mejor_reputacion(&self) -> Result <Vec <AccountId>, String>{
            let vendedores = self.filtrar_vendedores()?;
            let vec_contador: Vec<(AccountId, u8)> = self.contar_promedios_vendedor(vendedores)?;
            
            let top5: Vec<AccountId> = self.calcular_5_mejores(vec_contador)?;

            return Ok(top5);
        }

        #[ink(message)]
        pub fn compradores_mejor_reputacion(&self) -> Result<Vec<AccountId>, String>{
            let compradores = self.filtrar_compradores()?;
            let vec_contador: Vec<(AccountId, u8)> = self.contar_promedios_comprador(compradores)?;
            
            let top5: Vec<AccountId> = self.calcular_5_mejores(vec_contador)?;

            return Ok(top5);
        }
        
        fn calcular_5_mejores(&self, vec_contador: Vec<(AccountId, u8)>) -> Result<Vec<AccountId>, String>{
            let mut v = vec_contador;
            v.sort_by(|a, b| b.1.cmp(&a.1));
            let top: Vec<AccountId> = v.into_iter().take(5).map(|(acct, _score)| acct).collect();
            Ok(top)
        }

        fn contar_promedios_vendedor(&self, vendedores: Vec<Usuario>) -> Result<Vec<(AccountId, u8)>, String>{
            let mut vector_contador = Vec::new();

            vendedores
            .iter()
            .for_each(|vendedor|{
                let id = vendedor.id_usuario;
                let promedio = self.promedio_reputacion(vendedor.datos_vendedor.clone().unwrap().reputacion_como_vendedor);
                let dato: (AccountId, u8) = (id, promedio);
                vector_contador.push(dato);
            });

            return Ok(vector_contador);
        }

        fn contar_promedios_comprador(&self, comprador: Vec<Usuario>) -> Result<Vec<(AccountId, u8)>, String>{
            let mut vector_contador = Vec::new();

            comprador
            .iter()
            .for_each(|comprador|{
                let id = comprador.id_usuario;
                let promedio = self.promedio_reputacion(comprador.datos_comprador.clone().unwrap().reputacion_como_comprador);
                let dato: (AccountId, u8) = (id, promedio);
                vector_contador.push(dato);
            });

            return Ok(vector_contador);
        }

        fn filtrar_compradores(&self) -> Result<Vec<Usuario>,String> {
            let usuarios = self.marketplace.get_usuarios()?;
            let compradores: Vec<Usuario> = usuarios
                .into_iter()
                .filter(|usuario|{
                    (usuario.rol == Rol::Comp) || (usuario.rol == Rol::Ambos)
                })
                .collect();
            return Ok(compradores) 
        }


        fn filtrar_vendedores(&self) -> Result<Vec<Usuario>,String> {
            let usuarios = self.marketplace.get_usuarios()?;
            let vendedores: Vec<Usuario> = usuarios
                .into_iter()
                .filter(|usuario|{
                    (usuario.rol == Rol::Vend) || (usuario.rol == Rol::Ambos)
                })
                .collect();
            return Ok(vendedores) 
        }

        fn promedio_reputacion(&self, puntajes: Vec<u8>) -> u8 {
            let n = puntajes.len();
            if n == 0 {
                return 0;
            }
            let suma: u32 = puntajes.iter().fold(0u32, |acc, &p| acc.saturating_add(p as u32));
            let mitad = (n as u32) / 2;
            let avg_u32 = (suma.saturating_add(mitad)).checked_div(n as u32).unwrap_or(0);
            avg_u32.min(u32::from(u8::MAX)) as u8
       }

        

    }

}
