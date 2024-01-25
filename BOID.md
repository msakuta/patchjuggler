
# Boids (Bird-like objects)

[Boids](https://en.wikipedia.org/wiki/Boids) are very old and famous algorithm to immitate a flock of birds, or any animals behavior that tend to group together.
There are mainly 3 terms:

* Separation

Force applied by separation is

$$
f_{\alpha,i} = \begin{cases}
\alpha \sum_j \frac{p_i - p_j}{R_\alpha - ||p_i - p_j||} & \mathrm{if } ||p_i - p_j|| < R_\alpha \\
0 & \mathrm{otherwise}
\end{cases}
$$
where $p_i$ is the position of $i$-th object, $R_\alpha$ the maximum distance parameter, and $\alpha$ a factor of force applied by this term.

* Alignment

A tendency to align velocity to surrounding objects.
In our model, the 

$$
f_{\beta,i} = \begin{cases}
\beta \sum_j v_i - v_j & \mathrm{if } ||p_i - p_j|| < R_\beta \\
0 & \mathrm{otherwise}
\end{cases}
$$
wher $\beta$ is a factor parameter.

* Cohesion

A tendency that boids get close together.
It has opposite effect from separation, but cohesion should have larger radius than separation to make a stable distance.

$$
f_{\gamma,i} = \begin{cases}
\gamma \frac{1}{N} \sum_j -(p_i - p_j) & \mathrm{if } ||p_i - p_j|| < R_\gamma \\
0 & \mathrm{otherwise}
\end{cases}
$$
where $\gamma$ is a factor parameter.

## The fourth term (group separation)

The original idea presented by Craig Reynolds in 1986 had only 3 terms above, but actually it's not good with only these 3.
If you put a lot of boids in the simulation, they tend to clamp together and make a huge single flock, which is unrealistic, and more importantly, uninteresting.

So we introduce the 4th term that we call group separation.
It applies if the distance is farther than the cohesion, so that too many boids won't clamp together.

$$
f_{\delta,i} = \begin{cases}
\delta \sum_j \frac{p_i - p_j}{R_\delta - ||p_i - p_j||} & \mathrm{if } R_\gamma < ||p_i - p_j|| < R_\delta \\
0 & \mathrm{otherwise}
\end{cases}
$$
where $\delta$ is a parameter.

## Putting all together

We simply sum up all contributions from all terms to determine the force.

$$
f_i = f_{\alpha, i} + f_{\beta, i} + f_{\gamma, i} + f_{\delta, i}
$$

Then we apply the position and velocity update rule

$$
v_i \leftarrow v_i + f_i \\
p_i \leftarrow p_i + \Delta v_i
$$
where $\Delta$ is a delta-time parameter.
