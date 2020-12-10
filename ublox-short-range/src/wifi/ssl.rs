use atat::AtatClient;
use heapless::{consts, ArrayLength, String};
use crate::{
    error::Error, 
    UbloxClient,
    command::security::{
        *,
        types::*,
    },
    socket::SocketHandle,
};

pub trait SSL {
    fn import_certificate(
        &self,
        profile_id: u8,
        name: &str,
        certificate: &[u8],
    ) -> Result<(), Error>;
    fn import_root_ca(&self, profile_id: u8, name: &str, root_ca: &[u8]) -> Result<(), Error>;
    fn import_private_key(
        &self,
        profile_id: u8,
        name: &str,
        private_key: &[u8],
        password: Option<&str>,
    ) -> Result<(), Error>;
    fn enable_ssl(&self, socket: SocketHandle, profile_id: u8) -> Result<(), Error>;
}

impl<C, N, L> SSL for UbloxClient<C, N, L>
where
    C: atat::AtatClient,
    N: ArrayLength<Option<crate::sockets::SocketSetItem<L>>>,
    L: ArrayLength<u8>,
{
    fn import_certificate(
        &self,
        profile_id: u8,
        name: &str,
        certificate: &[u8],
    ) -> Result<(), Error> {
        assert!(name.len() < 200);

        self.send_at(PrepareSecurityDataImport {
            data_type: SecurityDataType::ClientCertificate,
            data_size: certificate.len(),
            internal_name: name,
            password: None,
        })?;

        self.send_at(SendSecurityDataImport {
            data: serde_at::ser::Bytes(certificate),
        })?;

        //Check MD5?

        Ok(())
    }

    fn import_root_ca(&self, profile_id: u8, name: &str, root_ca: &[u8]) -> Result<(), Error> {
        assert!(name.len() < 200);

        self.send_at(PrepareSecurityDataImport {
            data_type: SecurityDataType::TrustedRootCA,
            data_size: root_ca.len(),
            internal_name: name,
            password: None,
        })?;

        self.send_at(SendSecurityDataImport {
            data: serde_at::ser::Bytes(root_ca),
        })?;

        //Check MD5?

        Ok(())
    }

    fn import_private_key(
        &self,
        profile_id: u8,
        name: &str,
        private_key: &[u8],
        password: Option<&str>,
    ) -> Result<(), Error> {
        assert!(name.len() < 200);

        self.send_at(PrepareSecurityDataImport {
            data_type: SecurityDataType::ClientPrivateKey,
            data_size: private_key.len(),
            internal_name: name,
            password,
        })?;

        self.send_at(SendSecurityDataImport {
            data: serde_at::ser::Bytes(private_key),
        })?;

        //Check MD5?

        Ok(())
    }

    fn enable_ssl(&self, socket: SocketHandle, profile_id: u8) -> Result<(), Error> {
        //Change socket handle to do SSL now, 
        //Needs name of Certificates.

        Ok(())
    }
}