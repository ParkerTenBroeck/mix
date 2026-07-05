let

    helpers = rec {
        e = pow 10;
        e- = exp: e (-exp);
    }

fix = f:
  let 
    x = f x;
  in x;

  f = final: { x = 1; y = final.x + 10; z = final.y + 100; };

in fix f