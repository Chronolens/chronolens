

local = {
    "ajfoaijfaiofaofjas": 1,
    "ofajfaeslkfjalkfja": 2,
    "aljfa;lfjalfja;lfa": 3,
    "ajfoaijfaiofaobasf": 4,
    "ajfoaijfaiofaofafs": 5
}

remote = {
    "ajfoaijfaiofaofjas": 1,
    "eaeklfjaklfajelkfa": 6,
    "aljfa;lfjalfja;lfa": 3,
    "falejflaksejfaalal": 7,
    "falejflaksejffahsf": 8
}

def main():
    onlyLocal: list[str] = [];
    localAndRemote: list[str] = [];
    for localChecksum in local.keys():
        if remote.get(localChecksum):
            localAndRemote.append(localChecksum)
        else:
            onlyLocal.append(localChecksum)
    for key in localAndRemote:
        del remote[key]

    print("Local")
    print(onlyLocal)
    print("Local and remote")
    print(localAndRemote)
    print("Remote")
    print(remote)
    
main()
