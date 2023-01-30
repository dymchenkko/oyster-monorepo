
# Setup AWS EC2 AMI's and VPC for oyster
Following is the description of the process to perform preliminary setups including setting up Base Amazon Machine Images and VPC to run a provider. The AMI's and VPC setup by this tutorial is used by oyster to run enclaves for jobs in EC2 instances.

This tutuorial assumes Ubuntu 20.4+. If you have an older Ubuntu or a different distro, commands might need modification before use.

 
## Preliminaries

### Setup AWS profiles using the AWS CLI
This setup requires you to setup a named profile using AWS CLI 

 - To install AWS CLI on your system please follow ["Installing or updating the latest version of the AWS CLI"](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html)
 - Next configure the AWS CLI and setup a named profile by following ["Configuring the AWS CLI"](https://docs.aws.amazon.com/cli/latest/userguide/cli-chap-configure.html)

### Install Go
This project requires Go version 1.18.1+ to run, to install go on your system, run the following command

    sudo apt install golang-go
You can then check the version by running 

    go version

 ### Install Node.js
 If your system doesn't have Node.js installed, you can refer [here](https://nodejs.org/en/download/), you can also [install Node.js using the package manager](https://nodejs.org/en/download/package-manager/). This project uses node version 16.18.0+ and npm version 8.19.0+.

### Setup pulumi
Install pulumi on your system by running the installation script by running the command, you can also refer the official docs [here](https://www.pulumi.com/docs/get-started/aws/begin/#before-you-begin) 

    curl -fsSL https://get.pulumi.com | sh
Restart the shell, for it to take effect. 

## Setting up the VPC
### Step 1: Install the npm packages
Run the following command to install all the relevant npm modules and packages

    npm install
### Step 2: Set the AWS profile
To set the named AWS profile, as an environment variable. Run

    export AWS_PROFILE=/*profile name*/
### Step 3: Review the code file
Check the code file to review and possibly modify the regions that you want to launch VPC's to. 
 
### Step 4: Run the pulumi project
Perform a local login to the pulumi cli by running

    pulumi login -l

Run the following command

    pulumi up
Now when prompted create a new stack and name it. You can also set a passphrase if you wish to.
Next, it will prompt you `Do you want to perform this update?`, Select `yes` 

This will successfully setup the required VPC, subnet, security group etc. needed for running oyster as a provider. All resources will be tagged with **`project:oyster`** tag.

## Setting up default Amazon Machine Images
### Step 1: Setup the repository
Clone the repository containing code base to run the setup by running the following commands

    git clone git@github.com:marlinprotocol/oyster-setup-aws.git && cd oyster-setup-aws
### Step 2: Build the executable
Run the following commands

    go get && go build
### Step 3: Run the executable
The executable requires a few environment variables to run, to set those up run the following commands. 

Set the name of the key pair to be used, if the specified key and, the *.pem* file in the `.ssh` folder in your home directory, exist, then in that case the existing keypair would be used otherwise a new keypair with the name specified would be created. 

    export KEY=/*keyname*/
    
Set the AWS profile and the region of setup

    export PROFILE=/*profile*/
    export REGION=/*region*/
Now to run the executable

    ./OysterSetupAWS
This process takes a while to run, where it creates the base EC2 instance and creates AMI's from them, and then proceeds to terminate the EC2 instances. At the end of this, you will have two AMI's by the name of  **`MarlinLauncherx86_64`** for `x86_64` architecture and **`MarlinLauncherARM64`** for `arm_64` architecture. Both AMI's will be tagged by **`project:oyster`** tag.

### Step 4: Copy AMI's across regions
With the above mentioned steps you will have created two AMI's for architectures `arm_64` and `x86_64` respectively in the region specified by you. However, these AMI's can only be used in the region the were created in. To use the AMI's in a different region you will have to copy these AMI's as is, into the required regions. 
You can run a simple script provided in the project to do so. Run the following commands

    chmod +x ami-copy.sh
 Next run the script by passing in the `aws_profile`, `source_region`, followed by the `destination regions` like
 

    ./ami-copy.sh <profile> <source_region> <dest_region_1> <dest_region_2> <dest_region_3>...
This will copy both the AMI's to all the regions supplied.

 

